//! 参考 imgui-rs-vulkan-renderer

use crate::gui::mesh::{GuiMesh, ImGuiVertex};
use crate::pipeline_settings::{FrameSettings, PipelineSettings};
use crate::render_context::RenderContext;
use crate::renderer::swapchain::RhiSwapchain;
use ash::vk;
use image::EncodableLayout;
use shader_layout_macro::ShaderLayout;
use std::mem::offset_of;
use std::{cell::RefCell, rc::Rc};
use truvis_rhi::core::descriptor::RhiDescriptorSetLayout;
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::synchronize::RhiBufferBarrier;
use truvis_rhi::shader_cursor::ShaderCursor;
use truvis_rhi::{
    basic::color::LabelColor,
    core::{
        buffer::RhiBuffer,
        command_buffer::RhiCommandBuffer,
        command_queue::RhiSubmitInfo,
        descriptor_pool::{RhiDescriptorPool, RhiDescriptorPoolCreateInfo},
        image::RhiImage2D,
        shader::RhiShaderModule,
        texture::RhiTexture2D,
    },
    rhi::Rhi,
};

pub struct Gui {
    pub context: RefCell<imgui::Context>,
    pub platform: imgui_winit_support::WinitPlatform,

    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    _descriptor_set_layout: RhiDescriptorSetLayout<UiShaderLayout>,

    _descriptor_pool: Rc<RhiDescriptorPool>,

    _fonts_texture: RhiTexture2D,
    font_descriptor_set: vk::DescriptorSet,

    meshes: Vec<Option<GuiMesh>>,

    _device: Rc<RhiDevice>,

    _cmd: Option<RhiCommandBuffer>,
}

impl Drop for Gui {
    fn drop(&mut self) {
        log::info!("Destroying Gui");
        unsafe {
            // 销毁 pipeline
            self._device.destroy_pipeline(self.pipeline, None);
            // 销毁 pipeline layout
            self._device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

// constructor & getter
impl Gui {
    pub fn new(rhi: &Rhi, window: &winit::window::Window, pipeline_settings: &PipelineSettings) -> Self {
        let (mut imgui, platform) = Self::create_imgui(window);

        let descriptor_set_layout = RhiDescriptorSetLayout::<UiShaderLayout>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "[uipass]descriptor-set-layout",
        );
        let pipeline_layout = Self::create_pipeline_layout(rhi.device.handle(), descriptor_set_layout.handle());
        rhi.device.debug_utils().set_object_debug_name(pipeline_layout, "[uipass]pipeline-layout");
        let pipeline =
            Self::create_pipeline(rhi, pipeline_settings.color_format, pipeline_settings.depth_format, pipeline_layout);
        rhi.device.debug_utils().set_object_debug_name(pipeline, "[uipass]pipeline");

        let fonts_texture = {
            let fonts = imgui.fonts();
            let atlas_texture = fonts.build_rgba32_texture();

            let image = Rc::new(RhiImage2D::from_rgba8(
                rhi,
                atlas_texture.width,
                atlas_texture.height,
                atlas_texture.data,
                "imgui-fonts-image",
            ));
            RhiTexture2D::new(rhi, image, "imgui-fonts-texture")
        };

        imgui.fonts().tex_id = imgui::TextureId::from(Self::FONT_TEX_ID);

        let descriptor_pool = Rc::new(RhiDescriptorPool::new(
            rhi.device.clone(),
            Rc::new(RhiDescriptorPoolCreateInfo::new(
                vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                1,
                vec![vk::DescriptorPoolSize::default()
                    .descriptor_count(1)
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)],
            )),
            "imgui-descriptor-pool",
        ));

        let descriptor_set = unsafe {
            rhi.device()
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(descriptor_pool.handle())
                        .set_layouts(std::slice::from_ref(&descriptor_set_layout.handle())),
                )
                .unwrap()[0]
        };
        rhi.device.debug_utils().set_object_debug_name(descriptor_set, "[uipass]descriptor");

        // write
        {
            let writes = UiShaderLayout::font().write_image(
                descriptor_set,
                0,
                vec![fonts_texture.descriptor_image_info(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)],
            );
            rhi.device.write_descriptor_sets(&[writes]);
        }

        // TODO Textures::new()

        Self {
            context: RefCell::new(imgui),
            platform,

            pipeline,
            pipeline_layout,

            _descriptor_set_layout: descriptor_set_layout,
            _fonts_texture: fonts_texture,
            _descriptor_pool: descriptor_pool,
            font_descriptor_set: descriptor_set,

            meshes: (0..pipeline_settings.frames_in_flight).map(|_| None).collect(),

            _device: rhi.device.clone(),

            _cmd: None,
        }
    }

    pub fn update_delta_time(&mut self, duration: std::time::Duration) {
        self.context.get_mut().io_mut().update_delta_time(duration);
    }

    fn create_imgui(window: &winit::window::Window) -> (imgui::Context, imgui_winit_support::WinitPlatform) {
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
                data: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/mplus-1p-regular.ttf")),
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

    fn create_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::PipelineLayout {
        let push_const_range = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: size_of::<glam::Mat4>() as u32,
        }];

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_const_range);

        unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() }
    }

    fn create_pipeline(
        rhi: &Rhi,
        color_format: vk::Format,
        depth_format: vk::Format,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline {
        let vert_shader_module =
            RhiShaderModule::new(rhi.device.clone(), std::path::Path::new("shader/build/imgui/imgui.slang.spv"));
        let frag_shader_module =
            RhiShaderModule::new(rhi.device.clone(), std::path::Path::new("shader/build/imgui/imgui.slang.spv"));

        let shader_states_infos = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader_module.handle())
                .name(cstr::cstr!("vsmain")),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader_module.handle())
                .name(cstr::cstr!("psmain")),
        ];

        // 20 = R32G32 + R32G32 + R8G8B8A8
        let binding_desc = ImGuiVertex::vertex_input_bindings();
        let attribute_desc = ImGuiVertex::vertex_input_attributes();

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
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
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

        let color_attachment_formats = [color_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats)
            .depth_attachment_format(depth_format);

        let pipeline_info = pipeline_info.push_next(&mut rendering_info);

        let pipeline = unsafe {
            rhi.device
                .handle()
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };

        vert_shader_module.destroy();
        frag_shader_module.destroy();

        pipeline
    }
}

impl Gui {
    /// fonts atlas 使用的 texture id
    const FONT_TEX_ID: usize = usize::MAX;

    /// 接受 window 的事件
    pub fn handle_event<T>(&mut self, window: &winit::window::Window, event: &winit::event::Event<T>) {
        self.platform.handle_event(self.context.get_mut().io_mut(), window, event);
    }

    /// 内部的执行顺序
    /// - WinitPlatform::prepare_frame()
    /// - Context::new_frame()
    /// - 自定义：app::update_ui()
    /// - WinitPlatform::prepare_render()
    /// - Context::render()
    pub fn draw(
        &mut self,
        rhi: &Rhi,
        render_ctx: &mut RenderContext,
        swapchian: &RhiSwapchain,
        frame_settings: &FrameSettings,
        window: &winit::window::Window,
        f: impl FnOnce(&mut imgui::Ui),
    ) {
        // 看源码可知：imgui 可能会设定鼠标位置
        self.platform.prepare_frame(self.context.borrow_mut().io_mut(), window).unwrap();

        let mut temp_imgui = self.context.borrow_mut();
        let frame = temp_imgui.new_frame();
        f(frame);
        // 看源码可知：imgui 可能会因此鼠标指针
        self.platform.prepare_render(frame, window);
        let draw_data = temp_imgui.render();
        if draw_data.total_vtx_count == 0 {
            return;
        }

        let frame_index = render_ctx.current_frame_label();

        rhi.device.debug_utils().begin_queue_label(
            rhi.graphics_queue.handle(),
            "[ui-pass]create-mesh",
            LabelColor::COLOR_STAGE,
        );
        self.meshes[frame_index].replace(GuiMesh::from_draw_data(rhi, render_ctx, draw_data));
        rhi.device().debug_utils().end_queue_label(rhi.graphics_queue.handle());

        let cmd = render_ctx.alloc_command_buffer("uipass-render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[uipass]draw");
        self.record_cmd(
            render_ctx,
            swapchian,
            frame_settings,
            &cmd,
            self.meshes[frame_index].as_ref().unwrap(),
            draw_data,
        );
        cmd.end();

        render_ctx.graphics_queue().submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    // TODO imgui 自己有个 Texture<> 类型，可以作为 hash 容器
    /// 根据 imgui 传来的 texture id，找到对应的 descriptor set
    fn get_texture(&self, texture_id: imgui::TextureId) -> vk::DescriptorSet {
        if texture_id.id() == Self::FONT_TEX_ID {
            self.font_descriptor_set
        } else {
            unimplemented!()
        }
    }

    fn record_cmd(
        &self,
        render_ctx: &mut RenderContext,
        swapchain: &RhiSwapchain,
        frame_settings: &FrameSettings,
        cmd: &RhiCommandBuffer,
        mesh: &GuiMesh,
        draw_data: &imgui::DrawData,
    ) {
        let color_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(swapchain.current_present_image_view())
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let depth_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(render_ctx.depth_view().handle())
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let render_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(frame_settings.viewport_extent.into())
            .color_attachments(std::slice::from_ref(&color_attach_info))
            .depth_attachment(&depth_attach_info);

        let viewport = vk::Viewport {
            width: draw_data.framebuffer_scale[0] * draw_data.display_size[0],
            height: draw_data.framebuffer_scale[1] * draw_data.display_size[1],
            min_depth: 0.0,
            ..Default::default()
        };

        cmd.cmd_begin_rendering(&render_info);
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline);
        cmd.cmd_set_viewport(0, std::slice::from_ref(&viewport));
        let projection =
            glam::Mat4::orthographic_rh(0.0, draw_data.display_size[0], 0.0, draw_data.display_size[1], -1.0, 1.0);
        cmd.cmd_push_constants(self.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, projection.as_ref().as_bytes());
        cmd.cmd_bind_index_buffer(&mesh._index_buffer, 0, vk::IndexType::UINT16);
        cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&mesh.vertex_buffer), &[0]);

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        // 缓存之前已经加载过的 texture
        let mut last_texture_id: Option<imgui::TextureId> = None;
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
                                None,
                            );
                            last_texture_id = Some(texture_id);
                        }

                        cmd.draw_indexed(
                            count as u32,
                            index_offset + idx_offset as u32,
                            1,
                            0,
                            vertex_offset + vtx_offset as i32,
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
}

#[derive(ShaderLayout)]
struct UiShaderLayout {
    #[binding = 0]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    #[stage = "FRAGMENT"]
    _font: (),
}
