use ash::vk;
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
    render::{AppInitInfo, Timer},
    run::{run, App},
};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex
{
    pos: [f32; 4],
    color: [f32; 4],
}

const INDEX_DATA: [u32; 3] = [0u32, 1, 2];
const VERTEX_DATA: [Vertex; 3] = [
    Vertex {
        pos: [-1.0, 1.0, 0.0, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        pos: [1.0, 1.0, 0.0, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        pos: [0.0, -1.0, 0.0, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
];

struct HelloTriangle
{
    vertex_buffer: RhiBuffer,
    index_buffer: RhiBuffer,
    pipeline: RhiPipeline,
}

impl HelloTriangle
{
    fn init_buffer(rhi: &'static Rhi) -> (RhiBuffer, RhiBuffer)
    {
        let mut index_buffer = RhiBuffer::new_index_buffer(rhi, std::mem::size_of_val(&INDEX_DATA), "index-buffer");
        index_buffer.transfer_data(&INDEX_DATA);

        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(rhi, std::mem::size_of_val(&VERTEX_DATA), "vertex-buffer");
        vertex_buffer.transfer_data(&VERTEX_DATA);

        (vertex_buffer, index_buffer)
    }

    fn init_pipeline(rhi: &'static Rhi, render_context: &mut RenderContext) -> RhiPipeline
    {
        let extent = render_context.swapchain_extent();
        let pipeline = RhiPipelineTemplate {
            fragment_shader_path: Some("shader/hello_triangle/triangle.frag.spv".into()),
            vertex_shader_path: Some("shader/hello_triangle/triangle.vert.spv".into()),
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
        .create_pipeline(rhi, "");

        pipeline
    }

    fn my_update(&self, rhi: &'static Rhi, render_context: &mut RenderContext)
    {
        render_context.acquire_frame();

        let color_attach = <Self as App>::get_color_attachment(render_context.current_present_image_view());
        let depth_attach = <Self as App>::get_depth_attachment(render_context.depth_image_view);
        let render_info = <Self as App>::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );

        let mut cmd = RenderContext::alloc_command_buffer(render_context, "render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        {
            cmd.begin_rendering(&render_info);
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
        let pipeline = HelloTriangle::init_pipeline(rhi, render_context);
        let (vertex_buffer, index_buffer) = HelloTriangle::init_buffer(rhi);
        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
        }
    }
}

impl App for HelloTriangle
{
    fn update(&self, rhi: &'static Rhi, render_context: &mut RenderContext, _: &Timer)
    {
        self.my_update(rhi, render_context);
    }

    fn init(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self
    {
        log::info!("start.");
        HelloTriangle::new(rhi, render_context)
    }

    fn get_render_init_info() -> AppInitInfo
    {
        AppInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "hello-triangle".to_string(),
            enable_validation: true,
        }
    }
}

fn main()
{
    run::<HelloTriangle>();
}
