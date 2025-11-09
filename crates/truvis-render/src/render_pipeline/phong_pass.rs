use crate::renderer::frame_context::FrameContext;
use crate::{
    pipeline_settings::{FrameLabel, FrameSettings},
    renderer::{bindless::BindlessManager, fif_buffer::FifBuffers, gpu_scene::GpuScene, scene_manager::SceneManager},
};
use ash::vk;
use std::{cell::RefCell, mem::offset_of, rc::Rc};
use truvis_gfx::resources::special_buffers::vertex_buffer::VertexLayout;
use truvis_gfx::{
    basic::color::LabelColor,
    commands::command_buffer::CommandBuffer,
    pipelines::{
        graphics_pipeline::{GraphicsPipeline, GraphicsPipelineCreateInfo, PipelineLayout},
        rendering_info::RenderingInfo,
    },
    resources::special_buffers::structured_buffer::StructuredBuffer,
};
use truvis_model_manager::vertex::aos_3d::VertexLayoutAoS3D;
use truvis_shader_binding::shader;

pub struct PhongPass {
    pipeline: GraphicsPipeline,
    bindless_manager: Rc<RefCell<BindlessManager>>,
}
impl PhongPass {
    pub fn new(
        color_format: vk::Format,
        depth_format: vk::Format,
        bindless_manager: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        let mut ci = GraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong3d.vs.slang.spv", cstr::cstr!("main"));
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv", cstr::cstr!("main"));

        ci.vertex_binding(VertexLayoutAoS3D::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAoS3D::vertex_input_attributes());

        ci.attach_info(vec![color_format], Some(depth_format), None);
        ci.color_blend(
            vec![
                vk::PipelineColorBlendAttachmentState::default()
                    .blend_enable(false)
                    .color_write_mask(vk::ColorComponentFlags::RGBA),
            ],
            [0.0; 4],
        );

        let pipeline_layout = Rc::new(PipelineLayout::new(
            &[bindless_manager.borrow().bindless_descriptor_layout.handle()],
            &[vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(size_of::<shader::raster::PushConstants>() as u32)],
            "phong-pass",
        ));

        let d3_pipe = GraphicsPipeline::new(&ci, pipeline_layout, "phong-d3-pipe");

        Self {
            pipeline: d3_pipe,
            bindless_manager,
        }
    }

    fn bind(
        &self,
        cmd: &CommandBuffer,
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
        cmd: &CommandBuffer,
        per_frame_data: &StructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
        scene_mgr: &SceneManager,
        fif_buffers: &FifBuffers,
        frame_settings: &FrameSettings,
    ) {
        let frame_label = FrameContext::frame_label();
        let rendering_info = RenderingInfo::new(
            vec![fif_buffers.render_target_image_view(frame_label).handle()],
            Some(fif_buffers.depth_image_view().handle()),
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
        gpu_scene.draw(cmd, scene_mgr, &mut |ins_idx, submesh_idx| {
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
