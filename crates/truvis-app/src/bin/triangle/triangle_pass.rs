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
use truvis_model_manager::components::geometry::GeometrySoA3D;
use truvis_model_manager::vertex::soa_3d::VertexLayoutSoA3D;
use truvis_render::apis::render_pass::{RenderPass, RenderSubpass};
use truvis_render::core::frame_context::FrameContext;
use truvis_render::core::renderer::{RenderContext, RenderContextMut};
use truvis_render::pipeline_settings::{FrameLabel, FrameSettings};

const_map!(ShaderStage<GfxShaderStageInfo>: {
    Vertex: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        entry_point: c"vsmain",
        path: TruvisPath::shader_path("hello_triangle/triangle.slang"),
    },
    Fragment: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::FRAGMENT,
        entry_point: c"psmain",
        path: TruvisPath::shader_path("hello_triangle/triangle.slang"),
    },
});

pub struct TriangleSubpass {
    pipeline: GfxGraphicsPipeline,
    _pipeline_layout: Rc<GfxPipelineLayout>,
}
impl RenderSubpass for TriangleSubpass {}
impl TriangleSubpass {
    pub fn new(frame_settings: &FrameSettings) -> Self {
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

        let pipeline_layout = Rc::new(GfxPipelineLayout::new(&[], &[], "hello-triangle"));
        let pipeline = GfxGraphicsPipeline::new(&pipeline_ci, pipeline_layout.clone(), "hello-triangle-pipeline");

        Self {
            _pipeline_layout: pipeline_layout,
            pipeline,
        }
    }

    pub fn draw(
        &self,
        render_context: &RenderContext,
        cmd: &GfxCommandBuffer,
        frame_label: FrameLabel,
        frame_settings: &FrameSettings,
        shape: &GeometrySoA3D,
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

            shape.cmd_bind_index_buffer(cmd);
            shape.cmd_bind_vertex_buffers(cmd);
            cmd.draw_indexed(shape.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
    }
}

pub struct TrianglePass {
    triangle_pass: TriangleSubpass,
}

impl RenderPass for TrianglePass {}

impl TrianglePass {
    pub fn new(frame_settings: &FrameSettings) -> Self {
        let triangle_pass = TriangleSubpass::new(frame_settings);
        Self { triangle_pass }
    }

    pub fn render(
        &self,
        render_context: &RenderContext,
        render_context_mut: &mut RenderContextMut,
        shape: &GeometrySoA3D,
    ) {
        let frame_label = FrameContext::get().frame_label();
        let frame_settings = FrameContext::get().frame_settings();

        let render_target_texture = render_context
            .gfx_resource_manager
            .get_texture(render_context.fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();

        // render triangle
        {
            let cmd = render_context_mut.cmd_allocator.alloc_command_buffer("triangle");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "triangle");

            // 将 render target 从 general -> color attachment
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

            self.triangle_pass.draw(render_context, &cmd, frame_label, &frame_settings, shape);

            // 将 render target 从 color attachment -> general
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
