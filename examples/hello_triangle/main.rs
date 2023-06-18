use ash::{extensions::khr::Swapchain, vk};
use memoffset::offset_of;
use rust_vk::{
    render::{EngineInitInfo, Render},
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
    Vertex { pos: [-1.0, 1.0, 0.0, 1.0], color: [0.0, 1.0, 0.0, 1.0] },
    Vertex { pos: [1.0, 1.0, 0.0, 1.0], color: [0.0, 0.0, 1.0, 1.0] },
    Vertex { pos: [0.0, -1.0, 0.0, 1.0], color: [1.0, 0.0, 0.0, 1.0] },
];


fn main()
{
    Render::init(&EngineInitInfo { window_width: 800, window_height: 800, app_name: "hello-triangle".to_string() });

    log::info!("start.");

    let mut index_buffer = RhiBuffer::new_index_buffer(std::mem::size_of_val(&INDEX_DATA), Some("index-buffer"));
    index_buffer.transfer_data(&INDEX_DATA);

    let vertex_buffer = RhiBuffer::new_vertex_buffer(std::mem::size_of_val(&VERTEX_DATA), Some("vertex-buffer"));

    let extent = RenderContext::extent();
    let pipeline = RhiPipelineTemplate {
        fragment_shader_path: Some("examples/hello_triangle/frag.spv".into()),
        vertex_shader_path: Some("examples/hello_triangle/vert.spv".into()),
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
    .create_pipeline();


    WindowSystem::instance().render_loop(|| {
        RenderContext::acquire_frame();

        let rhi = Rhi::instance();

        let mut cmd = RhiCommandBuffer::new(rhi.graphics_command_pool());
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        {
            cmd.image_barrier(
                (vk::PipelineStageFlags::TOP_OF_PIPE, vk::AccessFlags::empty()),
                (vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
                RenderContext::current_image(),
                vk::ImageAspectFlags::COLOR,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            );

            cmd.begin_rendering(&RenderContext::render_info());
            cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &pipeline);
            cmd.bind_index_buffer(&index_buffer, 0, vk::IndexType::UINT32);
            cmd.bind_vertex_buffer(0, std::slice::from_ref(&vertex_buffer), &[0]);
            cmd.draw_indexed((INDEX_DATA.len() as u32, 0), (1, 0), 0);
            cmd.end_rendering();

            cmd.image_barrier(
                (vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
                (vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::AccessFlags::empty()),
                RenderContext::current_image(),
                vk::ImageAspectFlags::COLOR,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
        }
        cmd.end();
        rhi.graphics_queue().submit(
            vec![RhiSubmitBatch {
                command_buffers: vec![cmd],
                wait_info: vec![(
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    RenderContext::current_swapchain_available_semaphore(),
                )],
                signal_info: vec![RenderContext::current_image_render_finish_semaphore()],
            }],
            Some(RenderContext::current_fence().clone()),
        );

        RenderContext::submit_frame();
    });
}
