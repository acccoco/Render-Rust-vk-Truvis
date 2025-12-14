use std::rc::Rc;

use ash::vk;
use itertools::Itertools;

use truvis_crate_tools::const_map;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::barrier::GfxImageBarrier;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::layout::GfxVertexLayout;
use truvis_gfx::{
    commands::command_buffer::GfxCommandBuffer,
    pipelines::{
        graphics_pipeline::{GfxGraphicsPipeline, GfxGraphicsPipelineCreateInfo, GfxPipelineLayout},
        rendering_info::GfxRenderingInfo,
        shader::GfxShaderStageInfo,
    },
};
use truvis_model_manager::components::geometry::RtGeometry;
use truvis_model_manager::vertex::soa_3d::VertexLayoutSoA3D;
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_render_base::frame_context::FrameContext;
use truvis_render_base::pipeline_settings::{FrameLabel, FrameSettings};
use truvis_render_graph::apis::render_pass::{RenderPass, RenderSubpass};
use truvis_render_graph::render_context::{RenderContext, RenderContextMut};

const_map!(ShaderStage<GfxShaderStageInfo>: {
    Vertex: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        entry_point: c"vsmain",
        path: TruvisPath::shader_path("async_test/async_test.slang"),
    },
    Fragment: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::FRAGMENT,
        entry_point: c"psmain",
        path: TruvisPath::shader_path("async_test/async_test.slang"),
    },
});

pub struct AsyncSubpass {
    pipeline: GfxGraphicsPipeline,
    pipeline_layout: Rc<GfxPipelineLayout>,
}
impl RenderSubpass for AsyncSubpass {}
impl AsyncSubpass {
    pub fn new(bindless_manager: &BindlessManager, frame_settings: &FrameSettings) -> Self {
        let mut pipeline_ci = GfxGraphicsPipelineCreateInfo::default();
        pipeline_ci.shader_stages(ShaderStage::iter().map(|stage| stage.value().clone()).collect_vec());
        pipeline_ci.attach_info(vec![frame_settings.color_format], None, Some(vk::Format::UNDEFINED));
        pipeline_ci.vertex_binding(VertexLayoutSoA3D::vertex_input_bindings());
        pipeline_ci.vertex_attribute(VertexLayoutSoA3D::vertex_input_attributes());
        pipeline_ci.color_blend(
            vec![
                vk::PipelineColorBlendAttachmentState::default()
                    .blend_enable(false)
                    .color_write_mask(vk::ColorComponentFlags::RGBA),
            ],
            [0.0; 4],
        );

        // Bindless Layout
        let bindless_layout = &bindless_manager.bindless_descriptor_layout;

        // Push Constants
        let push_constant_range =
            vk::PushConstantRange::default().stage_flags(vk::ShaderStageFlags::ALL).offset(0).size(4); // uint texture_id

        let pipeline_layout =
            Rc::new(GfxPipelineLayout::new(&[bindless_layout.handle()], &[push_constant_range], "async-test"));

        let pipeline = GfxGraphicsPipeline::new(&pipeline_ci, pipeline_layout.clone(), "async-test-pipeline");

        Self {
            pipeline_layout,
            pipeline,
        }
    }

    pub fn draw(
        &self,
        render_context: &RenderContext,
        cmd: &GfxCommandBuffer,
        frame_label: FrameLabel,
        frame_settings: &FrameSettings,
        shape: &RtGeometry,
        texture_id: u32,
    ) {
        let viewport_extent = frame_settings.frame_extent;

        let render_target_texture = render_context
            .gfx_resource_manager
            .get_texture(render_context.fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();

        let rendering_info = GfxRenderingInfo::new(
            vec![render_target_texture.image_view().handle()],
            None,
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: viewport_extent,
            },
        );

        {
            cmd.cmd_begin_rendering2(&rendering_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());

            cmd.cmd_set_viewport(
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: viewport_extent.height as f32,
                    width: viewport_extent.width as f32,
                    height: -(viewport_extent.height as f32),
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            cmd.cmd_set_scissor(
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: viewport_extent,
                }],
            );

            // Bind Bindless Descriptor Set
            let bindless_set = render_context.bindless_manager.current_descriptor_set().handle();
            cmd.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout.handle(),
                0,
                &[bindless_set],
                None,
            );

            // Push Constants
            cmd.cmd_push_constants(
                self.pipeline_layout.handle(),
                vk::ShaderStageFlags::ALL,
                0,
                &texture_id.to_ne_bytes(),
            );

            shape.cmd_bind_index_buffer(cmd);
            shape.cmd_bind_vertex_buffers(cmd);
            cmd.draw_indexed(shape.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
    }
}

pub struct AsyncPass {
    async_pass: AsyncSubpass,
}

impl RenderPass for AsyncPass {}

impl AsyncPass {
    pub fn new(bindless_manager: &BindlessManager, frame_settings: &FrameSettings) -> Self {
        let async_pass = AsyncSubpass::new(bindless_manager, frame_settings);
        Self { async_pass }
    }

    pub fn render(
        &self,
        render_context: &RenderContext,
        render_context_mut: &mut RenderContextMut,
        shape: &RtGeometry,
        texture_id: u32,
    ) {
        let frame_label = FrameContext::get().frame_label();

        let render_target_texture = render_context
            .gfx_resource_manager
            .get_texture(render_context.fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();

        let frame_settings = FrameContext::get().frame_settings();

        // render
        {
            let cmd = render_context_mut.cmd_allocator.alloc_command_buffer("async-test");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "async-test");

            // Barrier: Render Target -> Color Attachment
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[GfxImageBarrier::new()
                    .image(render_target_texture.image().handle())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )
                    .dst_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )],
            );

            self.async_pass.draw(render_context, &cmd, frame_label, &frame_settings, shape, texture_id);

            // Barrier: Color Attachment -> General
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[GfxImageBarrier::new()
                    .image(render_target_texture.image().handle())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::GENERAL)
                    .src_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )
                    .dst_mask(vk::PipelineStageFlags2::NONE, vk::AccessFlags2::NONE)],
            );

            cmd.end();
            Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(&[cmd])], None);
        }
    }
}
