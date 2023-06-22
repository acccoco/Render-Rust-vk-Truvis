use ash::{extensions::khr::Swapchain, vk};
use memoffset::offset_of;
use rust_vk::{
    render::{RenderInitInfo, Render},
    render_context::RenderContext,
    resource_type::{
        buffer::RhiBuffer, command_buffer::RhiCommandBuffer, pipeline::RhiPipelineTemplate, queue::RhiSubmitBatch,
    },
    rhi::Rhi,
    window_system::WindowSystem,
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


fn main()
{
    Render::init(&RenderInitInfo {
        window_width: 800,
        window_height: 800,
        app_name: "hello-triangle".to_string(),
    });

    log::info!("start.");

    let mut index_buffer = RhiBuffer::new_index_buffer(std::mem::size_of_val(&INDEX_DATA), "index-buffer");
    index_buffer.transfer_data(&INDEX_DATA);

    let mut vertex_buffer = RhiBuffer::new_vertex_buffer(std::mem::size_of_val(&VERTEX_DATA), "vertex-buffer");
    vertex_buffer.transfer_data(&VERTEX_DATA);

    let extent = RenderContext::extent();
    let pipeline = RhiPipelineTemplate {
        fragment_shader_path: Some("examples/hello_triangle/shader/frag.spv".into()),
        vertex_shader_path: Some("examples/hello_triangle/shader/vert.spv".into()),
        color_formats: vec![RenderContext::instance().color_format()],
        depth_format: RenderContext::depth_format(),
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
        color_attach_blend_states: vec![vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .build()],
        ..Default::default()
    }
    .create_pipeline("");

    WindowSystem::instance().render_loop(|| {
        RenderContext::acquire_frame();

        let rhi = Rhi::instance();

        let mut cmd = RenderContext::get_command_buffer("render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        {
            cmd.begin_rendering(&RenderContext::render_info());
            cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &pipeline);
            cmd.bind_index_buffer(&index_buffer, 0, vk::IndexType::UINT32);
            cmd.bind_vertex_buffer(0, std::slice::from_ref(&vertex_buffer), &[0]);
            cmd.draw_indexed((INDEX_DATA.len() as u32, 0), (1, 0), 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue().submit(
            vec![RhiSubmitBatch {
                command_buffers: vec![cmd],
                ..Default::default()
            }],
            None,
        );

        RenderContext::submit_frame();
    });
}
