//! 参考 imgui-rs-vulkan-renderer

use std::{cell::RefCell, ffi::CString};

use ash::vk;
use image::EncodableLayout;
use imgui::TextureId;

use crate::framework::{
    basic::{color::RED, FRAME_ID_MAP},
    core::{
        buffer::RhiBuffer, command_buffer::RhiCommandBuffer, image::RhiImage2D, queue::RhiSubmitInfo,
        shader::RhiShaderModule, texture::RhiTexture,
    },
    rendering::render_context::RenderContext,
    rhi::Rhi,
};

pub struct UiMesh
{
    pub vertex: RhiBuffer,
    vertex_count: usize,

    pub indices: RhiBuffer,
    index_count: usize,
}

impl UiMesh
{
    // TODO 频繁的创建和销毁 buffer，性能不好
    pub fn from_draw_data(rhi: &'static Rhi, render_ctx: &mut RenderContext, draw_data: &imgui::DrawData) -> Self
    {
        rhi.graphics_queue_begin_label("uipass-create-mesh", RED);
        let mut cmd = render_ctx.alloc_command_buffer("uipass-create-mesh");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        let (vertex_buffer, vertex_cnt) = Self::create_vertices(rhi, render_ctx, &mut cmd, draw_data);
        let (index_buffer, index_cnt) = Self::create_indices(rhi, render_ctx, &mut cmd, draw_data);


        cmd.begin_label("uipass-mesh-transfer-barrier", RED);
        {
            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[vk::BufferMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .dst_stage_mask(vk::PipelineStageFlags2::INDEX_INPUT)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags2::INDEX_READ)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .buffer(index_buffer.handle)
                    .offset(0)
                    .size(vk::WHOLE_SIZE)],
            );

            cmd.buffer_memory_barrier(
                vk::DependencyFlags::empty(),
                &[vk::BufferMemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                    .dst_stage_mask(vk::PipelineStageFlags2::VERTEX_INPUT)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_access_mask(vk::AccessFlags2::VERTEX_ATTRIBUTE_READ)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .buffer(vertex_buffer.handle)
                    .offset(0)
                    .size(vk::WHOLE_SIZE)],
            );
        }
        cmd.end_label();
        cmd.end();

        rhi.graphics_queue_submit(
            vec![RhiSubmitInfo {
                command_buffers: vec![cmd],
                ..Default::default()
            }],
            None,
        );

        rhi.graphics_queue_end_label();

        Self {
            vertex: vertex_buffer,
            vertex_count: vertex_cnt,
            indices: index_buffer,
            index_count: index_cnt,
        }
    }

    /// # Return
    /// (vertices buffer, vertex count)
    fn create_vertices(
        rhi: &'static Rhi,
        render_ctx: &mut RenderContext,
        cmd: &mut RhiCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (RhiBuffer, usize)
    {
        let vertex_count = draw_data.total_vtx_count as usize;
        let mut vertices = Vec::with_capacity(vertex_count);
        for draw_list in draw_data.draw_lists() {
            vertices.extend_from_slice(draw_list.vtx_buffer());
        }

        let vertices_size = vertex_count * size_of::<imgui::DrawVert>();
        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(
            rhi,
            vertices_size,
            &format!("{}-imgui-vertex-buffer", render_ctx.current_frame_prefix()),
        );
        {
            // FIXME destroy stage buffer
            let mut stage_buffer = RhiBuffer::new_stage_buffer(
                rhi,
                vertices_size as vk::DeviceSize,
                &format!("{}-imgui-vertex-stage-buffer", render_ctx.current_frame_prefix()),
            );
            stage_buffer.transfer_data_by_mem_map(&vertices);

            cmd.begin_label("uipass-vertex-buffer-transfer", RED);
            {
                cmd.copy_buffer(
                    &stage_buffer,
                    &mut vertex_buffer,
                    &[vk::BufferCopy {
                        size: vertices_size as vk::DeviceSize,
                        ..Default::default()
                    }],
                );
            }
            cmd.end_label();
        }

        (vertex_buffer, vertex_count)
    }

    /// # Return
    /// (index buffer, index count)
    fn create_indices(
        rhi: &'static Rhi,
        render_ctx: &mut RenderContext,
        cmd: &mut RhiCommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> (RhiBuffer, usize)
    {
        let index_count = draw_data.total_idx_count as usize;
        let mut indices = Vec::with_capacity(index_count);
        for draw_list in draw_data.draw_lists() {
            indices.extend_from_slice(draw_list.idx_buffer());
        }

        let indices_size = index_count * std::mem::size_of::<imgui::DrawIdx>();
        let mut index_buffer = RhiBuffer::new_index_buffer(
            rhi,
            indices_size,
            &format!("{}-imgui-index-buffer", render_ctx.current_frame_prefix()),
        );
        {
            // FIXME destroy stage buffer
            let mut stage_buffer = RhiBuffer::new_stage_buffer(
                rhi,
                indices_size as vk::DeviceSize,
                &format!("{}-imgui-index-stage-buffer", render_ctx.current_frame_prefix()),
            );
            stage_buffer.transfer_data_by_mem_map(&indices);

            cmd.begin_label("uipass-index-buffer-transfer", RED);
            {
                cmd.copy_buffer(
                    &stage_buffer,
                    &mut index_buffer,
                    &[vk::BufferCopy {
                        size: indices_size as vk::DeviceSize,
                        ..Default::default()
                    }],
                );
            }
            cmd.end_label();
        }

        (index_buffer, index_count)
    }

    fn destroy(mut self)
    {
        self.indices.destroy();
        self.vertex.destroy();
    }
}


pub struct UI
{
    pub imgui: RefCell<imgui::Context>,
    pub platform: imgui_winit_support::WinitPlatform,

    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,

    descriptor_pool: vk::DescriptorPool,

    fonts_texture: RhiTexture,
    font_descriptor_set: vk::DescriptorSet,

    meshes: Vec<Option<UiMesh>>,
}

pub struct UiOptions
{
    pub frames_in_flight: usize,
}


impl UI
{
    /// fonts atlas 使用的 texture id
    const FONT_TEX_ID: usize = usize::MAX;


    pub fn new(
        rhi: &'static Rhi,
        render_ctx: &RenderContext,
        window: &winit::window::Window,
        options: &UiOptions,
    ) -> Self
    {
        let (mut imgui, platform) = Self::create_imgui(window);

        let descriptor_set_layout = Self::create_descriptor_set(&rhi.device.device);
        rhi.set_debug_name(descriptor_set_layout, "[uipass]descriptor-set-layout");
        let pipeline_layout = Self::create_pipeline_layout(&rhi.device.device, descriptor_set_layout);
        rhi.set_debug_name(pipeline_layout, "[uipass]pipeline-layout");
        let pipeline = Self::create_pipeline(rhi, render_ctx, pipeline_layout);
        rhi.set_debug_name(pipeline, "[uipass]pipeline");
        

        let fonts_texture = {
            let fonts = imgui.fonts();
            let atlas_texture = fonts.build_rgba32_texture();

            let image = RhiImage2D::from_rgba8(
                rhi,
                atlas_texture.width,
                atlas_texture.height,
                atlas_texture.data,
                "imgui-fonts-image",
            );
            RhiTexture::new(rhi, image, "imgui-fonts-texture")
        };

        let fonts = imgui.fonts();
        fonts.tex_id = imgui::TextureId::from(Self::FONT_TEX_ID);

        let pool_sizes =
            [vk::DescriptorPoolSize::default().descriptor_count(1).ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)];
        let descriptor_pool = rhi.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::default()
                .pool_sizes(&pool_sizes)
                .max_sets(1)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET),
            "imgui-descriptor-pool",
        );

        let descriptor_set = rhi.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(std::slice::from_ref(&descriptor_set_layout)),
        )[0];
        rhi.set_debug_name(descriptor_set, "[uipass]descriptor");

        // write
        {
            let image_info = vk::DescriptorImageInfo::default()
                .sampler(fonts_texture.sampler)
                .image_view(fonts_texture.image_view)
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            let writes = vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&image_info));
            rhi.write_descriptor_sets(std::slice::from_ref(&writes));
        }

        // TODO Textures::new()

        Self {
            imgui: RefCell::new(imgui),
            platform,

            pipeline,
            pipeline_layout,

            descriptor_set_layout,
            fonts_texture,
            descriptor_pool,
            font_descriptor_set: descriptor_set,

            meshes: (0..options.frames_in_flight).map(|_| None).collect(),
        }
    }

    pub fn draw(&mut self, rhi: &'static Rhi, render_ctx: &mut RenderContext) -> Option<RhiCommandBuffer>
    {
        let mut temp_imgui = self.imgui.borrow_mut();
        let draw_data = temp_imgui.render();
        if draw_data.total_vtx_count == 0 {
            return None;
        }

        let frame_index = render_ctx.current_frame_index();

        // TODO 这里需要标注一下名称，每个 tick 都会重新建立 vertex buffer
        if let Some(mesh) = self.meshes[frame_index].replace(UiMesh::from_draw_data(rhi, render_ctx, draw_data)) {
            mesh.destroy()
        }

        let mut cmd = render_ctx.alloc_command_buffer("uipass-render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        cmd.begin_label("[uipass]draw", RED);
        self.record_cmd(render_ctx, &mut cmd, self.meshes[frame_index].as_ref().unwrap(), draw_data);
        cmd.end_label();
        cmd.end();

        Some(cmd)
    }

    // TODO imgui 自己有个 Texture<> 类型，可以作为 hash 容器
    /// 根据 imgui 传来的 texture id，找到对应的 descriptor set
    fn get_texture(&self, texture_id: imgui::TextureId) -> vk::DescriptorSet
    {
        if texture_id.id() == Self::FONT_TEX_ID {
            self.font_descriptor_set
        } else {
            unimplemented!()
        }
    }

    fn record_cmd(
        &self,
        render_ctx: &mut RenderContext,
        cmd: &mut RhiCommandBuffer,
        mesh: &UiMesh,
        draw_data: &imgui::DrawData,
    )
    {
        let color_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(render_ctx.current_present_image_view())
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let depth_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(render_ctx.depth_image_view)
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let render_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(render_ctx.swapchain_extent().into())
            .color_attachments(std::slice::from_ref(&color_attach_info))
            .depth_attachment(&depth_attach_info);

        let viewport = vk::Viewport {
            width: draw_data.framebuffer_scale[0] * draw_data.display_size[0],
            height: draw_data.framebuffer_scale[1] * draw_data.display_size[1],
            min_depth: 0.0,
            ..Default::default()
        };

        cmd.cmd_begin_rendering(&render_info);
        cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        cmd.cmd_set_viewport(0, std::slice::from_ref(&viewport));
        let projection =
            glam::Mat4::orthographic_rh(0.0, draw_data.display_size[0], 0.0, draw_data.display_size[1], -1.0, 1.0);
        cmd.cmd_push_constants(self.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, projection.as_ref().as_bytes());
        cmd.bind_index_buffer(&mesh.indices, 0, vk::IndexType::UINT16);
        cmd.bind_vertex_buffer(0, std::slice::from_ref(&mesh.vertex), &[0]);

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        // 缓存之前已经加载过的 texture
        let mut last_texture_id: Option<TextureId> = None;
        let clip_offset = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        // 简而言之：对于每个 command，设置正确的 vertex, index, texture, scissor 即可
        for draw_list in draw_data.draw_lists() {
            for command in draw_list.commands() {
                match command {
                    imgui::DrawCmd::Elements {
                        count,
                        cmd_params:
                            imgui::DrawCmdParams {
                                clip_rect,
                                texture_id, // 当前绘制命令用到的 texture，这个 id 是 app 决定的
                                vtx_offset,
                                idx_offset,
                            },
                    } => {
                        let clip_x = (clip_rect[0] - clip_offset[0]) * clip_scale[0];
                        let clip_y = (clip_rect[1] - clip_offset[1]) * clip_scale[1];
                        let clip_w = (clip_rect[2] - clip_offset[0]) * clip_scale[0] - clip_x;
                        let clip_h = (clip_rect[3] - clip_offset[1]) * clip_scale[1] - clip_y;

                        let scissors = [vk::Rect2D {
                            offset: vk::Offset2D {
                                x: (clip_x as i32).max(0),
                                y: (clip_y as i32).max(0),
                            },
                            extent: vk::Extent2D {
                                width: clip_w as _,
                                height: clip_h as _,
                            },
                        }];
                        cmd.cmd_set_scissor(0, &scissors);

                        // 加载 texture，如果和上一个 command 使用的 texture 不是同一个，则需要重新加载
                        if Some(texture_id) != last_texture_id {
                            cmd.bind_descriptor_sets(
                                vk::PipelineBindPoint::GRAPHICS,
                                self.pipeline_layout,
                                0,
                                &[self.get_texture(texture_id)],
                                &[],
                            );
                            last_texture_id = Some(texture_id);
                        }

                        cmd.draw_indexed2(
                            count as u32,
                            1,
                            index_offset + idx_offset as u32,
                            vertex_offset + vtx_offset as i32,
                            0,
                        );
                    }
                    imgui::DrawCmd::ResetRenderState => {
                        log::warn!("imgui reset render state");
                    }
                    imgui::DrawCmd::RawCallback { .. } => {
                        log::warn!("imgui raw callback");
                    }
                }
            }

            index_offset += draw_list.idx_buffer().len() as u32;
            vertex_offset += draw_list.vtx_buffer().len() as i32;
        }
        cmd.end_rendering();
    }

    fn create_imgui(window: &winit::window::Window) -> (imgui::Context, imgui_winit_support::WinitPlatform)
    {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None); // disable automatic saving .ini file
        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui);

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    size_pixels: font_size,
                    ..Default::default()
                }),
            },
            imgui::FontSource::TtfData {
                data: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts/mplus-1p-regular.ttf")),
                size_pixels: font_size,
                config: Some(imgui::FontConfig {
                    rasterizer_multiply: 1.75,
                    glyph_ranges: imgui::FontGlyphRanges::japanese(),
                    ..Default::default()
                }),
            },
        ]);
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        platform.attach_window(imgui.io_mut(), window, imgui_winit_support::HiDpiMode::Rounded);

        (imgui, platform)
    }

    // TODO refactor
    fn create_descriptor_set(device: &ash::Device) -> vk::DescriptorSetLayout
    {
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];

        let descriptor_set_create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        unsafe { device.create_descriptor_set_layout(&descriptor_set_create_info, None).unwrap() }
    }


    fn create_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::PipelineLayout
    {
        let push_const_range = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: size_of::<glam::Mat4>() as u32,
        }];

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_const_range);
        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() };
        pipeline_layout
    }

    fn create_pipeline(
        rhi: &'static Rhi,
        render_ctx: &RenderContext,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline
    {
        let entry_point_name = CString::new("main").unwrap();

        let vert_shader_module = RhiShaderModule::new(rhi, std::path::Path::new("shader/imgui/shader.vs.hlsl.spv"));
        let frag_shader_module = RhiShaderModule::new(rhi, std::path::Path::new("shader/imgui/shader.ps.hlsl.spv"));

        let shader_states_infos = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader_module.handle)
                .name(&entry_point_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader_module.handle)
                .name(&entry_point_name),
        ];

        // 20 = R32G32 + R32G32 + R8G8B8A8
        let binding_desc = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(20)
            .input_rate(vk::VertexInputRate::VERTEX)];
        let attribute_desc = [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM)
                .offset(16),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_desc)
            .vertex_attribute_descriptions(&attribute_desc);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let viewports = [Default::default()];
        let scissors = [Default::default()];
        let viewport_info = vk::PipelineViewportStateCreateInfo::default().viewports(&viewports).scissors(&scissors);

        let multisampling_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1) // fixme msaa 1
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::ColorComponentFlags::R |
                    vk::ColorComponentFlags::G |
                    vk::ColorComponentFlags::B |
                    vk::ColorComponentFlags::A,
            )
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)];
        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false) // FIXME
            .depth_write_enable(false) // FIXME
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let dynamic_states = [vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT];
        let dynamic_states_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_states_infos)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .rasterization_state(&rasterizer_info)
            .viewport_state(&viewport_info)
            .multisample_state(&multisampling_info)
            .color_blend_state(&color_blending_info)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .dynamic_state(&dynamic_states_info)
            .layout(pipeline_layout)
            .subpass(0);

        let color_attachment_formats = [render_ctx.color_format()];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats)
            .depth_attachment_format(render_ctx.depth_format);

        let pipeline_info = pipeline_info.push_next(&mut rendering_info);

        let pipeline = unsafe {
            rhi.device
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };

        vert_shader_module.destroy();
        frag_shader_module.destroy();

        pipeline
    }
}
