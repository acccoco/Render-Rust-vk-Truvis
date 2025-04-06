use std::mem::offset_of;

use ash::vk;
use bytemuck::{Pod, Zeroable};
use imgui::Ui;
use truvis_render::{
    framework::rendering::render_context::RenderContext,
    render::{App, AppCtx, AppInitInfo, Renderer, Timer},
};
use truvis_rhi::{
    core::{
        buffer::RhiBuffer,
        command_queue::RhiSubmitInfo,
        pipeline::{RhiPipeline, RhiPipelineTemplate},
    },
    render_core::Rhi,
};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex
{
    pos: [f32; 4],
    color: [f32; 4],
}

const INDEX_DATA: [u32; 6] = [0u32, 1, 2, 0, 2, 3];
const VERTEX_DATA: [Vertex; 4] = [
    // left bottom
    Vertex {
        pos: [-1.0, 1.0, 0.0, 1.0],
        color: [0.2, 0.2, 0.0, 1.0],
    },
    // right bottom
    Vertex {
        pos: [1.0, 1.0, 0.0, 1.0],
        color: [0.8, 0.2, 0.0, 1.0],
    },
    // right top
    Vertex {
        pos: [1.0, -1.0, 0.0, 1.0],
        color: [0.8, 0.8, 0.0, 1.0],
    },
    // left top
    Vertex {
        pos: [-1.0, -1.0, 0.0, 1.0],
        color: [0.2, 0.8, 0.0, 1.0],
    },
];


#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
pub struct PushConstants
{
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


struct ShaderToy
{
    vertex_buffer: RhiBuffer,
    index_buffer: RhiBuffer,
    pipeline: RhiPipeline,
}

impl ShaderToy
{
    fn init_buffer(rhi: &Rhi) -> (RhiBuffer, RhiBuffer)
    {
        let mut index_buffer = RhiBuffer::new_index_buffer(rhi, size_of_val(&INDEX_DATA), "index-buffer");
        index_buffer.transfer_data_by_stage_buffer(rhi, &INDEX_DATA);

        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(rhi, size_of_val(&VERTEX_DATA), "vertex-buffer");
        vertex_buffer.transfer_data_by_stage_buffer(rhi, &VERTEX_DATA);

        (vertex_buffer, index_buffer)
    }

    fn init_pipeline(rhi: &Rhi, render_context: &RenderContext) -> RhiPipeline
    {
        let extent = render_context.swapchain_extent();
        let push_constant_ranges = vec![vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::ALL,
            offset: 0,
            size: size_of::<PushConstants>() as u32,
        }];
        RhiPipelineTemplate {
            vertex_shader_path: Some("shader/shadertoy-glsl/shadertoy.vert.spv".into()),
            fragment_shader_path: Some("shader/shadertoy-glsl/shadertoy.frag.spv".into()),
            color_formats: vec![render_context.color_format()],
            depth_format: render_context.depth_format(),
            viewport: Some(vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as _,
                height: extent.height as _,
                min_depth: 0.0,
                max_depth: 1.0,
            }),
            scissor: Some(extent.into()),
            vertex_binding_desc: vec![vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }],
            push_constant_ranges,
            vertex_attribute_desec: vec![
                vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Vertex, pos) as u32,
                },
                vk::VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Vertex, color) as u32,
                },
            ],
            color_attach_blend_states: vec![vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)],
            ..Default::default()
        }
        .create_pipeline(rhi, "shadertoy")
    }

    fn run(&self, rhi: &Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        let push_constants = PushConstants {
            time: timer.total_time,
            delta_time: timer.delta_time,
            frame: timer.total_frame,
            frame_rate: 1.0 / timer.delta_time,
            resolution: glam::Vec2::new(
                render_context.swapchain_extent().width as f32,
                render_context.swapchain_extent().height as f32,
            ),
            mouse: glam::Vec4::new(
                0.2 * (render_context.swapchain_extent().width as f32),
                0.2 * (render_context.swapchain_extent().height as f32),
                0.0,
                0.0,
            ),
            __padding__: [0.0, 0.0],
        };

        let depth_attach_info = <Self as App>::get_depth_attachment(render_context.depth_view.handle());
        let color_attach_info = <Self as App>::get_color_attachment(render_context.current_present_image_view());
        let render_info = <Self as App>::get_render_info(
            vk::Rect2D {
                offset: Default::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach_info),
            &depth_attach_info,
        );

        let cmd = render_context.alloc_command_buffer("render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[main-pass]draw");
        {
            cmd.cmd_push_constants(
                self.pipeline.pipeline_layout,
                vk::ShaderStageFlags::ALL,
                0,
                bytemuck::bytes_of(&push_constants),
            );

            cmd.cmd_begin_rendering(&render_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
            cmd.cmd_bind_index_buffer(&self.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.vertex_buffer), &[0]);
            cmd.draw_indexed(INDEX_DATA.len() as u32, 0, 1, 0, 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn new(rhi: &Rhi, render_context: &mut RenderContext) -> Self
    {
        log::info!("start.");

        let (vertex_buffer, index_buffer) = Self::init_buffer(rhi);
        let pipeline = Self::init_pipeline(rhi, render_context);

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
        }
    }
}

impl App for ShaderToy
{
    fn update_ui(&mut self, ui: &mut Ui)
    {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, _app_ctx: &mut AppCtx)
    {
        //
    }

    fn draw(&self, app_ctx: &mut AppCtx)
    {
        self.run(app_ctx.rhi, app_ctx.render_context, app_ctx.timer)
    }

    fn init(rhi: &Rhi, render_context: &mut RenderContext) -> Self
    {
        ShaderToy::new(rhi, render_context)
    }

    fn get_render_init_info() -> AppInitInfo
    {
        AppInitInfo {
            window_width: 1600,
            window_height: 900,
            app_name: "hello-triangle".to_string(),
            enable_validation: true,
        }
    }
}

fn main()
{
    Renderer::<ShaderToy>::run();
}
