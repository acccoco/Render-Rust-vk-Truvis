use ash::vk;
use rust_vk::{
    engine::{Engine, EngineInitInfo},
    rhi_resource::buffer::RhiBuffer,
};

fn main()
{
    Engine::init(&EngineInitInfo {
        window_width: 800,
        window_height: 800,
        app_name: "hello-triangle".to_string(),
    });

    log::info!("start.");


    unsafe {
        let index_buffer_data = [0u32, 1, 2];

        let mut stage_buffer = RhiBuffer::new_index_buffer(
            std::mem::size_of_val(&index_buffer_data) as vk::DeviceSize,
            Some("index-buffer-stage-buffer"),
        );

        let mut index_buffer = RhiBuffer::new_index_buffer(
            std::mem::size_of_val(&index_buffer_data) as vk::DeviceSize,
            Some("index-buffer"),
        );

        //
        // let mut stage_buffer = HissBuffer::new_stage_buffer(
        //     engine.core().clone(),
        //     "index stage buffer",
        //     std::mem::size_of_val(&index_buffer_data) as u64,
        // );
        // stage_buffer.map_data_slice(&index_buffer_data, 0);
        // let mut index_buffer =
        //     HissBuffer::new_index_buffer(engine.core().clone(), "index buffer", *stage_buffer.size());
        // index_buffer.transfer_data(stage_buffer);
        //
        // let vertices = [
        //     Vertex {
        //         pos: [-1.0, 1.0, 0.0, 1.0],
        //         color: [0.0, 1.0, 0.0, 1.0],
        //     },
        //     Vertex {
        //         pos: [1.0, 1.0, 0.0, 1.0],
        //         color: [0.0, 0.0, 1.0, 1.0],
        //     },
        //     Vertex {
        //         pos: [0.0, -1.0, 0.0, 1.0],
        //         color: [1.0, 0.0, 0.0, 1.0],
        //     },
        // ];
        //
        // let mut stage_buffer = HissBuffer::new_stage_buffer(
        //     engine.core().clone(),
        //     "vertex stage buffer",
        //     std::mem::size_of_val(&vertices) as u64,
        // );
        // stage_buffer.map_data_slice(&vertices, 0);
        // let mut vertex_buffer =
        //     HissBuffer::new_vertex_buffer(engine.core().clone(), "vertex buffer", *stage_buffer.size());
        // vertex_buffer.transfer_data(stage_buffer);
        //
        // // depth image
        // let depth_image = HissImage::builder()
        //     .core(engine.core().clone())
        //     .name("depth image")
        //     .format(vk::Format::D16_UNORM)
        //     .extent(*engine.surface_resolution())
        //     .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        //     .aspect(vk::ImageAspectFlags::DEPTH)
        //     .init_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        //     .build();
        //
        // let layout_create_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&[]);
        //
        // let pipeline_layout = engine.core().device().create_pipeline_layout(&layout_create_info, None).unwrap();
        //
        // let vert_shader = engine.core().load_shader(Path::new(&format!("{CUR_DIR}/shader/vert.spv")));
        // let frag_shader = engine.core().load_shader(Path::new(&format!("{CUR_DIR}/shader/frag.spv")));
        // let shaders = [vert_shader, frag_shader];
        //
        // let framebuffers: Vec<_> = engine
        //     .present_image_views()
        //     .iter()
        //     .map(|view| HissFramebuffer::new(&[*view], depth_image.image_view(), *engine.surface_resolution()))
        //     .collect();
        //
        // let render_infos: Vec<_> = framebuffers.iter().map(|framebuffer| framebuffer.render_info()).collect();
        //
        // // let render_infos =
        //
        // let graphics_pipeline = HissPipelineTemplate::default()
        //     .shaders([
        //         (vk::ShaderStageFlags::VERTEX, shaders[0]),
        //         (vk::ShaderStageFlags::FRAGMENT, shaders[1]),
        //     ])
        //     .color_formats([engine.surface_format().format])
        //     .depth_format(vk::Format::D16_UNORM)
        //     .viewport((*engine.surface_resolution()).into())
        //     .vertex_binding_state([vk::VertexInputBindingDescription {
        //         binding: 0,
        //         stride: std::mem::size_of::<Vertex>() as u32,
        //         input_rate: vk::VertexInputRate::VERTEX,
        //     }])
        //     .vertex_attribute_state([
        //         vk::VertexInputAttributeDescription {
        //             location: 0,
        //             binding: 0,
        //             format: vk::Format::R32G32B32A32_SFLOAT,
        //             offset: offset_of!(Vertex, pos) as u32,
        //         },
        //         vk::VertexInputAttributeDescription {
        //             location: 1,
        //             binding: 0,
        //             format: vk::Format::R32G32B32A32_SFLOAT,
        //             offset: offset_of!(Vertex, color) as u32,
        //         },
        //     ])
        //     .color_blend_attach_state([vk::PipelineColorBlendAttachmentState::builder()
        //         .blend_enable(false)
        //         .color_write_mask(vk::ColorComponentFlags::RGBA)
        //         .build()])
        //     .generate(engine.core().as_ref(), &pipeline_layout);
        //
        // engine.render_loop(|| {
        //     // 从 present engine 获取图像
        //     let (present_index, _) = engine
        //         .swapchain_loader()
        //         .acquire_next_image(
        //             *engine.swapchain(),
        //             u64::MAX,
        //             *engine.present_complete_semaphore(),
        //             vk::Fence::null(),
        //         )
        //         .unwrap();
        //
        //     record_submit_command(
        //         engine.core().clone().device(),
        //         *engine.draw_command_buffer(),
        //         *engine.draw_commands_reuse_fence(),
        //         *engine.queue(),
        //         &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
        //         &[*engine.present_complete_semaphore()],
        //         &[*engine.rendering_complete_semaphore()],
        //         |device, draw_command_buffer| {
        //             // color 图像布局转换
        //             barrier::image_barrier(
        //                 device,
        //                 draw_command_buffer,
        //                 vk::AccessFlags::empty(),
        //                 vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        //                 vk::PipelineStageFlags::TOP_OF_PIPE,
        //                 vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        //                 engine.present_images()[present_index as usize],
        //                 vk::ImageAspectFlags::COLOR,
        //                 vk::ImageLayout::UNDEFINED,
        //                 vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        //             );
        //
        //             // 开始绘制
        //             engine
        //                 .dynamic_render_loader()
        //                 .cmd_begin_rendering(draw_command_buffer, &render_infos[present_index as usize]);
        //             {
        //                 device.cmd_bind_pipeline(
        //                     draw_command_buffer,
        //                     vk::PipelineBindPoint::GRAPHICS,
        //                     graphics_pipeline,
        //                 );
        //
        //                 device.cmd_bind_vertex_buffers(draw_command_buffer, 0, &[*vertex_buffer.buffer()], &[0]);
        //                 device.cmd_bind_index_buffer(
        //                     draw_command_buffer,
        //                     *index_buffer.buffer(),
        //                     0,
        //                     vk::IndexType::UINT32,
        //                 );
        //                 device.cmd_draw_indexed(draw_command_buffer, index_buffer_data.len() as u32, 1, 0, 0, 1);
        //             }
        //             engine.dynamic_render_loader().cmd_end_rendering(draw_command_buffer);
        //
        //             // color 图像布局变换
        //             barrier::image_barrier(
        //                 device,
        //                 draw_command_buffer,
        //                 vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        //                 vk::AccessFlags::empty(),
        //                 vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        //                 vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        //                 engine.present_images()[present_index as usize],
        //                 vk::ImageAspectFlags::COLOR,
        //                 vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        //                 vk::ImageLayout::PRESENT_SRC_KHR,
        //             );
        //         },
        //     );
        //     //let mut present_info_err = mem::zeroed();
        //     let wait_semaphors = [*engine.rendering_complete_semaphore()];
        //     let swapchains = [*engine.swapchain()];
        //     let image_indices = [present_index];
        //     let present_info = vk::PresentInfoKHR::builder()
        //         .wait_semaphores(&wait_semaphors) // &base.rendering_complete_semaphore)
        //         .swapchains(&swapchains)
        //         .image_indices(&image_indices);
        //
        //     engine.swapchain_loader().queue_present(*engine.queue(), &present_info).unwrap();
        // });
        //
        // engine.core().clone().device().device_wait_idle().unwrap();
        // engine.core().device().destroy_pipeline(graphics_pipeline, None);
        // engine.core().device().destroy_pipeline_layout(pipeline_layout, None);
        // engine.core().device().destroy_shader_module(shaders[0], None);
        // engine.core().device().destroy_shader_module(shaders[1], None);
    }
}
