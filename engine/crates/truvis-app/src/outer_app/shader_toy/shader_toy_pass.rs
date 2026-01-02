use std::rc::Rc;

use ash::vk;
use bytemuck::{Pod, Zeroable};
use itertools::Itertools;

use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::enumed_map;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::barrier::GfxImageBarrier;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::layout::GfxVertexLayout;
use truvis_gfx::resources::vertex_layout::soa_3d::VertexLayoutSoA3D;
use truvis_gfx::{
    commands::command_buffer::GfxCommandBuffer,
    pipelines::{
        graphics_pipeline::{GfxGraphicsPipeline, GfxGraphicsPipelineCreateInfo, GfxPipelineLayout},
        rendering_info::GfxRenderingInfo,
        shader::GfxShaderStageInfo,
    },
};
use truvis_render_graph::apis::render_pass::{RenderPass, RenderSubpass};
use truvis_render_graph::render_context::RenderContext;
use truvis_render_interface::cmd_allocator::CmdAllocator;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::geometry::RtGeometry;
use truvis_render_interface::pipeline_settings::FrameSettings;

enumed_map!(ShaderStage<GfxShaderStageInfo>:{
    Vertex: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        entry_point: c"main",
        path: TruvisPath::shader_build_path_str("shadertoy-glsl/shadertoy.vert"),
    },
    Fragment: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::FRAGMENT,
        entry_point: c"main",
        path: TruvisPath::shader_build_path_str("shadertoy-glsl/shadertoy.frag"),
    },
});

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct PushConstants {
    /// 鼠标位置和状态
    mouse: glam::Vec4,
    /// 分辨率
    resolution: glam::Vec2,
    /// 播放时间 seconds
    time: f32,
    /// frame 渲染时间 seconds
    delta_time: f32,
    /// 累计渲染帧数
    frame: i32,
    /// 帧率
    frame_rate: f32,
    /// padding
    __padding__: [f32; 2],
}

pub struct ShaderToySubpass {
    pipeline: GfxGraphicsPipeline,
    _pipeline_layout: Rc<GfxPipelineLayout>,
}
impl RenderSubpass for ShaderToySubpass {}
impl ShaderToySubpass {
    pub fn new(color_format: vk::Format) -> Self {
        let mut pipeline_ci = GfxGraphicsPipelineCreateInfo::default();
        pipeline_ci.shader_stages(ShaderStage::iter().map(|stage| stage.value().clone()).collect_vec());
        pipeline_ci.attach_info(vec![color_format], None, Some(vk::Format::UNDEFINED));
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

        let pipeline_layout = Rc::new(GfxPipelineLayout::new(
            &[],
            &[vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<PushConstants>() as u32,
            }],
            "shader-toy",
        ));
        let pipeline = GfxGraphicsPipeline::new(&pipeline_ci, pipeline_layout.clone(), "shader-toy");

        Self {
            _pipeline_layout: pipeline_layout,
            pipeline,
        }
    }

    pub fn draw(
        &self,
        render_context: &RenderContext,
        cmd: &GfxCommandBuffer,
        frame_settings: &FrameSettings,
        render_target: vk::ImageView,
        rect: &RtGeometry,
    ) {
        let viewport_extent = frame_settings.frame_extent;

        let push_constants = PushConstants {
            time: render_context.total_time_s,
            delta_time: render_context.delta_time_s,
            frame: render_context.frame_counter.frame_id() as i32,
            frame_rate: 1.0 / render_context.delta_time_s,
            resolution: glam::Vec2::new(viewport_extent.width as f32, viewport_extent.height as f32),
            mouse: glam::Vec4::new(
                0.2 * (viewport_extent.width as f32),
                0.2 * (viewport_extent.height as f32),
                0.0,
                0.0,
            ),
            __padding__: [0.0, 0.0],
        };

        let rendering_info = GfxRenderingInfo::new(
            vec![render_target],
            None,
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: viewport_extent,
            },
        );

        {
            cmd.cmd_push_constants(
                self.pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&push_constants),
            );

            cmd.cmd_begin_rendering2(&rendering_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());

            cmd.cmd_set_viewport(
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: viewport_extent.width as f32,
                    height: viewport_extent.height as f32,
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

            rect.cmd_bind_index_buffer(cmd);
            rect.cmd_bind_vertex_buffers(cmd);
            cmd.draw_indexed(rect.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
    }
}

pub struct ShaderToyPass {
    shader_toy_pass: ShaderToySubpass,

    shader_toy_cmds: [GfxCommandBuffer; FrameCounter::fif_count()],
}

impl RenderPass for ShaderToyPass {}

impl ShaderToyPass {
    pub fn new(color_format: vk::Format, cmd_allocator: &mut CmdAllocator) -> Self {
        let shader_toy_pass = ShaderToySubpass::new(color_format);
        let shader_toy_cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "shader-toy"));
        Self {
            shader_toy_pass,
            shader_toy_cmds,
        }
    }

    pub fn render(&self, render_context: &RenderContext, shape: &RtGeometry) {
        let frame_label = render_context.frame_counter.frame_label();

        let render_target_texture = render_context
            .gfx_resource_manager
            .get_texture(render_context.fif_buffers.render_target_texture_handle(frame_label))
            .unwrap();

        // render shader toy
        {
            let cmd = self.shader_toy_cmds[*frame_label].clone();
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "shader-toy");

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

            self.shader_toy_pass.draw(
                render_context,
                &cmd,
                &render_context.frame_settings,
                render_target_texture.image_view().handle(),
                shape,
            );

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
