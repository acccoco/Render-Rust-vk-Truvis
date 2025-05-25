use crate::frame_context::FrameContext;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_scene::GpuScene;
use ash::vk;
use model_manager::vertex::vertex_3d::VertexLayoutAos3D;
use model_manager::vertex::VertexLayout;
use shader_binding::shader;
use std::cell::RefCell;
use std::mem::offset_of;
use std::rc::Rc;
use truvis_rhi::basic::color::LabelColor;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::graphics_pipeline::{RhiGraphicsPipeline, RhiGraphicsPipelineCreateInfo};
use truvis_rhi::rhi::Rhi;

pub struct PhongPass {
    pipeline: RhiGraphicsPipeline,
    bindless_manager: Rc<RefCell<BindlessManager>>,
}
impl PhongPass {
    pub fn new(rhi: &Rhi, frame_context: &FrameContext, bindless_manager: Rc<RefCell<BindlessManager>>) -> Self {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong3d.vs.slang.spv", cstr::cstr!("main"));
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv", cstr::cstr!("main"));

        ci.vertex_binding(VertexLayoutAos3D::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAos3D::vertex_input_attributes());

        ci.push_constant_ranges(vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<shader::PushConstants>() as u32)]);
        ci.descriptor_set_layouts(vec![bindless_manager.borrow().bindless_layout.handle()]);
        ci.attach_info(vec![frame_context.color_format()], Some(frame_context.depth_format()), None);
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        let d3_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-d3-pipe");

        Self {
            pipeline: d3_pipe,
            bindless_manager,
        }
    }

    fn bind(
        &self,
        cmd: &RhiCommandBuffer,
        viewport: &vk::Rect2D,
        push_constant: &shader::PushConstants,
        frame_idx: usize,
    ) {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline());
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: viewport.offset.x as f32,
                y: viewport.offset.y as f32 + viewport.extent.height as f32,
                width: viewport.extent.width as f32,
                height: -(viewport.extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(0, &[*viewport]);
        cmd.cmd_push_constants(
            self.pipeline.layout(),
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constant),
        );

        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.layout(),
            0,
            &[self.bindless_manager.borrow().bindless_sets[frame_idx].handle()],
            &[],
        );
    }

    pub fn draw(
        &self,
        cmd: &RhiCommandBuffer,
        render_info: &vk::RenderingInfo,
        viewport: vk::Extent2D,
        per_frame_data: &RhiStructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
        frame_label: usize,
    ) {
        cmd.cmd_begin_rendering(render_info);
        cmd.begin_label("[phong-pass]draw", LabelColor::COLOR_PASS);

        self.bind(
            cmd,
            &viewport.into(),
            &shader::PushConstants {
                frame_data: per_frame_data.device_address(),
                scene: gpu_scene.scene_device_address(frame_label),
                ..Default::default()
            },
            frame_label,
        );
        gpu_scene.draw(cmd, &mut |ins_idx, submesh_idx| {
            // NOTE 这个数据和 PushConstant 中的内存布局是一致的
            let data = [ins_idx, submesh_idx];
            cmd.cmd_push_constants(
                self.pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset_of!(shader::PushConstants, instance_idx) as u32,
                bytemuck::bytes_of(&data),
            );
        });

        cmd.end_label();
        cmd.end_rendering();
    }
}
