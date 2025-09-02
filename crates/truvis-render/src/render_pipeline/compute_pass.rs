use std::{ffi::CStr, rc::Rc};

use ash::vk;
use truvis_rhi::{
    commands::command_buffer::CommandBuffer, foundation::device::DeviceFunctions,
    pipelines::shader::ShaderModule, render_context::RenderContext,
};

use crate::renderer::bindless::BindlessManager;

/// 泛型参数 P 表示 compute shader 的参数，以 push constant 的形式传入 shader
pub struct ComputePass<P: bytemuck::Pod>
{
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    _phantom: std::marker::PhantomData<P>,

    device_functions: Rc<DeviceFunctions>,
}
impl<P: bytemuck::Pod> ComputePass<P>
{
    pub fn new(rhi: &RenderContext, bindless_mgr: &BindlessManager, entry_point: &CStr, shader_path: &str) -> Self
    {
        let shader_module = ShaderModule::new(rhi, std::path::Path::new(shader_path));
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .module(shader_module.handle())
            .stage(vk::ShaderStageFlags::COMPUTE)
            .name(entry_point);

        let pipeline_layout = {
            let push_constant_range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
                .offset(0)
                .size(size_of::<P>() as u32);

            let descriptor_sets = [bindless_mgr.bindless_descriptor_layout.handle()];
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_sets)
                .push_constant_ranges(std::slice::from_ref(&push_constant_range));

            unsafe { rhi.device_functions().create_pipeline_layout(&pipeline_layout_ci, None).unwrap() }
        };

        let pipeline_ci = vk::ComputePipelineCreateInfo::default().stage(stage_info).layout(pipeline_layout);
        let pipeline = unsafe {
            rhi.device_functions()
                .create_compute_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_ci), None)
                .unwrap()[0]
        };

        shader_module.destroy();

        Self {
            pipeline,
            pipeline_layout,

            _phantom: std::marker::PhantomData,
            device_functions: rhi.device_functions().clone(),
        }
    }

    pub fn exec(&self, cmd: &CommandBuffer, bindless_mgr: &BindlessManager, params: &P, group_cnt: glam::UVec3)
    {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::COMPUTE, self.pipeline);

        cmd.cmd_push_constants(self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, bytemuck::bytes_of(params));
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline_layout,
            0,
            &[bindless_mgr.current_descriptor_set().handle()],
            None,
        );

        // 执行计算
        cmd.cmd_dispatch(group_cnt);
    }
}
impl<P: bytemuck::Pod> Drop for ComputePass<P>
{
    fn drop(&mut self)
    {
        unsafe {
            self.device_functions.destroy_pipeline(self.pipeline, None);
            self.device_functions.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
