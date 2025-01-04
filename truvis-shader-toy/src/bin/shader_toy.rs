use ash::vk;
use bytemuck::{Pod, Zeroable};
use memoffset::offset_of;
use truvis_render::{
    framework::{
        core::{
            buffer::RhiBuffer,
            pipeline::{RhiPipeline, RhiPipelineTemplate},
            queue::RhiSubmitBatch,
        },
        rendering::render_context::RenderContext,
        rhi::Rhi,
    },
    render::{RenderInitInfo, Timer},
    run::{run, App},
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
    fn init_buffer(rhi: &'static Rhi) -> (RhiBuffer, RhiBuffer)
    {
        let mut index_buffer = RhiBuffer::new_index_buffer(rhi, std::mem::size_of_val(&INDEX_DATA), "index-buffer");
        index_buffer.transfer_data(&INDEX_DATA);

        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(rhi, std::mem::size_of_val(&VERTEX_DATA), "vertex-buffer");
        vertex_buffer.transfer_data(&VERTEX_DATA);

        (vertex_buffer, index_buffer)
    }

    fn init_pipeline(rhi: &'static Rhi, render_context: &RenderContext) -> RhiPipeline
    {
        let extent = render_context.extent();
        let push_constant_ranges = vec![vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::ALL,
            offset: 0,
            size: std::mem::size_of::<PushConstants>() as u32,
        }];
        let pipeline = RhiPipelineTemplate {
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
                stride: std::mem::size_of::<Vertex>() as u32,
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
            color_attach_blend_states: vec![vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .build()],
            ..Default::default()
        }
        .create_pipeline(rhi, "");

        pipeline
    }

    fn run(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        render_context.acquire_frame();

        let push_constants = PushConstants {
            time: timer.total_time,
            delta_time: timer.delta_time,
            frame: timer.total_frame,
            frame_rate: 1.0 / timer.delta_time,
            resolution: glam::Vec2::new(render_context.extent().width as f32, render_context.extent().height as f32),
            mouse: glam::Vec4::new(
                0.2 * (render_context.extent().width as f32),
                0.2 * (render_context.extent().height as f32),
                0.0,
                0.0,
            ),
            __padding__: [0.0, 0.0],
        };

        let mut cmd = render_context.alloc_command_buffer("render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        {
            cmd.push_constants(&self.pipeline, vk::ShaderStageFlags::ALL, 0, bytemuck::bytes_of(&push_constants));

            cmd.begin_rendering(&render_context.render_info());
            cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &self.pipeline);
            cmd.bind_index_buffer(&self.index_buffer, 0, vk::IndexType::UINT32);
            cmd.bind_vertex_buffer(0, std::slice::from_ref(&self.vertex_buffer), &[0]);
            cmd.draw_indexed((INDEX_DATA.len() as u32, 0), (1, 0), 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue().submit(
            rhi,
            vec![RhiSubmitBatch {
                command_buffers: vec![cmd],
                ..Default::default()
            }],
            None,
        );

        render_context.submit_frame();
    }

    fn new(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self
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
    fn init(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self
    {
        ShaderToy::new(rhi, render_context)
    }

    fn get_render_init_info() -> RenderInitInfo
    {
        RenderInitInfo {
            window_width: 1600,
            window_height: 900,
            app_name: "hello-triangle".to_string(),
        }
    }

    fn update(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        self.run(rhi, render_context, timer)
    }
}

fn main()
{
    run::<ShaderToy>();
}
