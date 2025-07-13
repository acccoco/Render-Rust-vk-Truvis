use crate::pipeline_settings::{FrameLabel, FrameSettings};
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_buffers::FrameBuffers;
use crate::renderer::frame_controller::FrameController;
use crate::renderer::gpu_scene::GpuScene;
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
use truvis_rhi::core::graphics_pipeline::{RhiGraphicsPipeline, RhiGraphicsPipelineCreateInfo, RhiPipelineLayout};
use truvis_rhi::core::rendering_info::RhiRenderingInfo;
use truvis_rhi::rhi::Rhi;

pub struct PhongPass {
    pipeline: RhiGraphicsPipeline,
    bindless_manager: Rc<RefCell<BindlessManager>>,
}
impl PhongPass {
    pub fn new(rhi: &Rhi, frame_settings: &FrameSettings, bindless_manager: Rc<RefCell<BindlessManager>>) -> Self {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong3d.vs.slang.spv", cstr::cstr!("main"));
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv", cstr::cstr!("main"));

        ci.vertex_binding(VertexLayoutAos3D::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAos3D::vertex_input_attributes());

        ci.attach_info(vec![frame_settings.color_format], Some(frame_settings.depth_format), None);
        ci.color_blend(
            vec![vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)],
            [0.0; 4],
        );

        let pipeline_layout = Rc::new(RhiPipelineLayout::new(
            rhi.device.clone(),
            &[bindless_manager.borrow().bindless_descriptor_layout.handle()],
            &[vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(size_of::<shader::raster::PushConstants>() as u32)],
            "phong-pass",
        ));

        let d3_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, pipeline_layout, "phong-d3-pipe");

        Self {
            pipeline: d3_pipe,
            bindless_manager,
        }
    }

    fn bind(
        &self,
        cmd: &RhiCommandBuffer,
        viewport: &vk::Rect2D,
        push_constant: &shader::raster::PushConstants,
        frame_idx: FrameLabel,
    ) {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());
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
            &[self.bindless_manager.borrow().bindless_descriptor_sets[*frame_idx].handle()],
            None,
        );
    }

    pub fn draw(
        &self,
        cmd: &RhiCommandBuffer,
        frame_ctx: &FrameController,
        per_frame_data: &RhiStructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
        frame_buffers: &FrameBuffers,
        frame_settings: &FrameSettings,
    ) {
        let frame_label = frame_ctx.frame_label();
        let rendering_info = RhiRenderingInfo::new(
            vec![frame_buffers.render_target_image_view(frame_label).handle()],
            Some(frame_buffers.depth_image_view().handle()),
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: frame_settings.frame_extent,
            },
        );

        cmd.cmd_begin_rendering2(&rendering_info);
        cmd.begin_label("[phong-pass]draw", LabelColor::COLOR_PASS);

        self.bind(
            cmd,
            &frame_settings.frame_extent.into(),
            &shader::raster::PushConstants {
                frame_data: per_frame_data.device_address(),
                scene: gpu_scene.scene_device_address(frame_label),

                submesh_idx: 0,  // 这个值在 draw 时会被更新
                instance_idx: 0, // 这个值在 draw 时会被更新

                _padding_1: Default::default(),
                _padding_2: Default::default(),
            },
            frame_label,
        );
        gpu_scene.draw(cmd, &mut |ins_idx, submesh_idx| {
            // NOTE 这个数据和 PushConstant 中的内存布局是一致的
            let data = [ins_idx, submesh_idx];
            cmd.cmd_push_constants(
                self.pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset_of!(shader::raster::PushConstants, instance_idx) as u32,
                bytemuck::bytes_of(&data),
            );
        });

        cmd.end_label();
        cmd.end_rendering();
    }
}
