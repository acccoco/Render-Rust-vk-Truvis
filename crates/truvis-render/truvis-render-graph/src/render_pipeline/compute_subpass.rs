use std::ffi::CStr;

use crate::apis::render_pass::RenderSubpass;
use crate::render_context::RenderContext;
use ash::vk;
use truvis_gfx::{commands::command_buffer::GfxCommandBuffer, gfx::Gfx, pipelines::shader::GfxShaderModule};
use truvis_render_base::render_descriptor_sets::RenderDescriptorSets;

/// 泛型参数 P 表示 compute shader 的参数，以 push constant 的形式传入 shader
pub struct ComputeSubpass<P: bytemuck::Pod> {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    _phantom: std::marker::PhantomData<P>,
}
impl<P: bytemuck::Pod> ComputeSubpass<P> {
    pub fn new(render_descriptor_sets: &RenderDescriptorSets, entry_point: &CStr, shader_path: &str) -> Self {
        let shader_module = GfxShaderModule::new(std::path::Path::new(shader_path));
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .module(shader_module.handle())
            .stage(vk::ShaderStageFlags::COMPUTE)
            .name(entry_point);

        let pipeline_layout = {
            let push_constant_range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(size_of::<P>() as u32);

            let descriptor_sets = [
                render_descriptor_sets.layout_0_global.handle(),
                render_descriptor_sets.layout_1_bindless.handle(),
            ];
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_sets)
                .push_constant_ranges(std::slice::from_ref(&push_constant_range));

            unsafe { Gfx::get().gfx_device().create_pipeline_layout(&pipeline_layout_ci, None).unwrap() }
        };

        let pipeline_ci = vk::ComputePipelineCreateInfo::default().stage(stage_info).layout(pipeline_layout);
        let pipeline = unsafe {
            Gfx::get()
                .gfx_device()
                .create_compute_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_ci), None)
                .unwrap()[0]
        };

        shader_module.destroy();

        Self {
            pipeline,
            pipeline_layout,

            _phantom: std::marker::PhantomData,
        }
    }

    pub fn exec(&self, cmd: &GfxCommandBuffer, render_context: &RenderContext, params: &P, group_cnt: glam::UVec3) {
        let frame_label = render_context.frame_counter.frame_label();
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::COMPUTE, self.pipeline);

        cmd.cmd_push_constants(self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, bytemuck::bytes_of(params));
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline_layout,
            0,
            &[
                render_context.render_descriptor_sets.set_0_global.handle(),
                render_context.render_descriptor_sets.current_bindless_descriptor_set(frame_label).handle(),
            ],
            None,
        );

        // 执行计算
        cmd.cmd_dispatch(group_cnt);
    }

    pub fn destroy(self) {
        // drop
    }
}
impl<P: bytemuck::Pod> RenderSubpass for ComputeSubpass<P> {}
impl<P: bytemuck::Pod> Drop for ComputeSubpass<P> {
    fn drop(&mut self) {
        let gfx_device = Gfx::get().gfx_device();
        unsafe {
            gfx_device.destroy_pipeline(self.pipeline, None);
            gfx_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
