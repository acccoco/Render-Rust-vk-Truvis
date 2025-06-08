//! 参考 imgui-rs-vulkan-renderer

use crate::gui::gui_pass::GuiPass;
use crate::gui::mesh::GuiMesh;
use crate::pipeline_settings::{FrameSettings, PipelineSettings};
use crate::render_context::RenderContext;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::swapchain::RhiSwapchain;
use ash::vk;
use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::{
    basic::color::LabelColor,
    core::{command_buffer::RhiCommandBuffer, command_queue::RhiSubmitInfo, image::RhiImage2D, texture::RhiTexture2D},
    rhi::Rhi,
};

pub struct Gui {
    pub imgui_ctx: RefCell<imgui::Context>,
    pub platform: imgui_winit_support::WinitPlatform,

    gui_pass: GuiPass,

    /// 从 imgui 内部的 texture id 到 bindless manager 中 texture 的映射
    textures_map: HashMap<imgui::TextureId, String>,
    fonts_texture_id: imgui::TextureId,
    fonts_texture_key: String,

    meshes: Vec<Option<GuiMesh>>,

    _device: Rc<RhiDevice>,

    _cmd: Option<RhiCommandBuffer>,
}
impl Drop for Gui {
    fn drop(&mut self) {}
}
// ctor
impl Gui {
    pub fn new(
        rhi: &Rhi,
        window: &winit::window::Window,
        pipeline_settings: &PipelineSettings,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        let mut imgui = imgui::Context::create();
        // disable automatic saving .ini file
        imgui.set_ini_filename(None);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui);
        platform.attach_window(imgui.io_mut(), window, imgui_winit_support::HiDpiMode::Rounded);

        let gui_pass = GuiPass::new(rhi, pipeline_settings, bindless_mgr.clone());

        let (fonts_texture_id, fonts_texture_key) =
            Self::init_fonts(rhi, &mut imgui, &platform, &mut bindless_mgr.borrow_mut());

        Self {
            imgui_ctx: RefCell::new(imgui),
            platform,

            gui_pass,
            textures_map: HashMap::new(),
            fonts_texture_id,
            fonts_texture_key,

            meshes: (0..pipeline_settings.frames_in_flight).map(|_| None).collect(),

            _device: rhi.device.clone(),

            _cmd: None,
        }
    }

    /// 初始化的时候注册字体
    ///
    /// 1. 首先将字体数据放入 imgui 中，并建立起字体 atlas
    /// 1. 然后将字体 atlas 转换为 RhiImage2D，并注册到 BindlessManager 中
    ///
    /// # Return
    /// ```
    /// (
    ///     "font texture id in imgui",
    ///     "font texture key in bindless manager"
    /// )
    /// ```
    fn init_fonts(
        rhi: &Rhi,
        imgui_ctx: &mut imgui::Context,
        platform: &imgui_winit_support::WinitPlatform,
        bindless_mgr: &mut BindlessManager,
    ) -> (imgui::TextureId, String) {
        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;

        imgui_ctx.fonts().add_font(&[
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
        imgui_ctx.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        let fonts_texture = {
            let fonts = imgui_ctx.fonts();
            let atlas_texture = fonts.build_rgba32_texture();

            let image = Rc::new(RhiImage2D::from_rgba8(
                rhi,
                atlas_texture.width,
                atlas_texture.height,
                atlas_texture.data,
                "imgui-fonts",
            ));
            RhiTexture2D::new(rhi, image, "imgui-fonts")
        };

        let fonts_texture_key = "imgui-fonts".to_string();
        let fonts_texture_id = imgui::TextureId::from(0);
        bindless_mgr.register_texture(fonts_texture_key.clone(), fonts_texture);
        imgui_ctx.fonts().tex_id = fonts_texture_id;

        (fonts_texture_id, fonts_texture_key)
    }
}
impl Gui {
    /// 接受 window 的事件
    pub fn handle_event<T>(&mut self, window: &winit::window::Window, event: &winit::event::Event<T>) {
        self.platform.handle_event(self.imgui_ctx.get_mut().io_mut(), window, event);
    }

    /// # Phase: IO
    /// 1. 可能会修改鼠标位置
    /// 1. 更新 imgui 的 delta time
    pub fn prepare_frame(&mut self, window: &winit::window::Window, duration: std::time::Duration) {
        // 看源码可知：imgui 可能会设定鼠标位置
        self.platform.prepare_frame(self.imgui_ctx.borrow_mut().io_mut(), window).unwrap();

        self.imgui_ctx.get_mut().io_mut().update_delta_time(duration);
    }

    /// # Phase: Update
    pub fn update(&mut self, window: &winit::window::Window, ui_func: impl FnOnce(&mut imgui::Ui)) {
        let mut imgui_ctx = self.imgui_ctx.borrow_mut();
        let frame = imgui_ctx.new_frame();
        ui_func(frame);
        // 看源码可知：imgui 可能会隐藏鼠标指针
        self.platform.prepare_render(frame, window);
    }

    /// # Phase: Render
    pub fn render(
        &mut self,
        rhi: &Rhi,
        render_ctx: &mut RenderContext,
        swapchian: &RhiSwapchain,
        frame_settings: &FrameSettings,
    ) {
        let mut imgui_ctx = self.imgui_ctx.borrow_mut();
        let draw_data = imgui_ctx.render();
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
        self.gui_pass.draw(
            render_ctx,
            swapchian,
            frame_settings,
            &cmd,
            self.meshes[frame_index].as_ref().unwrap(),
            draw_data,
            |texture_id| self.get_texture(texture_id),
        );
        cmd.end();

        render_ctx.graphics_queue().submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    /// 根据 imgui 传来的 texture id，找到对应的 texture key，用于在 bindless manager 中得到 texture
    fn get_texture(&self, texture_id: imgui::TextureId) -> String {
        if texture_id == self.fonts_texture_id {
            self.fonts_texture_key.clone()
        } else {
            self.textures_map.get(&texture_id).unwrap().clone()
        }
    }
}
