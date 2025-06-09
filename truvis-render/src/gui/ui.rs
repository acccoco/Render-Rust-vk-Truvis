//! 参考 imgui-rs-vulkan-renderer

use crate::gui::mesh::GuiMesh;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::FrameContext;
use crate::renderer::pipeline_settings::RendererSettings;
use std::{cell::RefCell, rc::Rc};
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::{
    basic::color::LabelColor,
    core::{image::RhiImage2D, texture::RhiTexture2D},
    rhi::Rhi,
};

pub struct Gui {
    pub imgui_ctx: imgui::Context,
    pub platform: imgui_winit_support::WinitPlatform,

    /// 存放多帧 imgui 的 mesh 数据
    meshes: Vec<Option<GuiMesh>>,

    _device: Rc<RhiDevice>,
}
impl Drop for Gui {
    fn drop(&mut self) {}
}
// ctor
impl Gui {
    const FONT_TEXTURE_ID: usize = 0;

    pub fn new(
        rhi: &Rhi,
        window: &winit::window::Window,
        renderer_settings: &RendererSettings,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        let mut imgui_ctx = imgui::Context::create();
        // disable automatic saving .ini file
        imgui_ctx.set_ini_filename(None);

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui_ctx);
        platform.attach_window(imgui_ctx.io_mut(), window, imgui_winit_support::HiDpiMode::Rounded);

        Self::init_fonts(rhi, &mut imgui_ctx, &platform, &mut bindless_mgr.borrow_mut());

        Self {
            imgui_ctx,
            platform,

            meshes: (0..renderer_settings.pipeline_settings.frames_in_flight).map(|_| None).collect(),

            _device: rhi.device.clone(),
        }
    }

    /// 初始化的时候注册字体
    ///
    /// 1. 首先将字体数据放入 imgui 中，并建立起字体 atlas
    /// 1. 然后将字体 atlas 转换为 RhiImage2D，并注册到 BindlessManager 中
    ///
    /// # Return
    /// ```
    ///     "font texture id in imgui"
    /// ```
    fn init_fonts(
        rhi: &Rhi,
        imgui_ctx: &mut imgui::Context,
        platform: &imgui_winit_support::WinitPlatform,
        bindless_mgr: &mut BindlessManager,
    ) {
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

        let fonts_texture_id = imgui::TextureId::from(Self::FONT_TEXTURE_ID);
        let fonts_texture_key = Self::get_texture_key(fonts_texture_id);
        bindless_mgr.register_texture(fonts_texture_key.clone(), fonts_texture);
        imgui_ctx.fonts().tex_id = fonts_texture_id;
    }
}
impl Gui {
    /// 接受 window 的事件
    pub fn handle_event<T>(&mut self, window: &winit::window::Window, event: &winit::event::Event<T>) {
        self.platform.handle_event(self.imgui_ctx.io_mut(), window, event);
    }

    /// # Phase: IO
    /// 1. 可能会修改鼠标位置
    /// 1. 更新 imgui 的 delta time
    pub fn prepare_frame(&mut self, window: &winit::window::Window, duration: std::time::Duration) {
        // 看源码可知：imgui 可能会设定鼠标位置
        self.platform.prepare_frame(self.imgui_ctx.io_mut(), window).unwrap();

        self.imgui_ctx.io_mut().update_delta_time(duration);
    }

    /// # Phase: Update
    pub fn update(&mut self, window: &winit::window::Window, ui_func: impl FnOnce(&mut imgui::Ui)) {
        let frame = self.imgui_ctx.new_frame();
        ui_func(frame);
        // 看源码可知：imgui 可能会隐藏鼠标指针
        self.platform.prepare_render(frame, window);
    }

    /// # Phase: Render
    ///
    /// 使用 imgui 将 ui 操作编译为 draw data；构建 draw 需要的 mesh 数据
    pub fn imgui_render(
        &mut self,
        rhi: &Rhi,
        cmd: &RhiCommandBuffer,
        render_ctx: &mut FrameContext,
    ) -> Option<(&GuiMesh, &imgui::DrawData)> {
        let draw_data = self.imgui_ctx.render();
        if draw_data.total_vtx_count == 0 {
            return None;
        }

        let frame_label = render_ctx.crt_frame_label();

        rhi.device.debug_utils().begin_queue_label(
            rhi.graphics_queue.handle(),
            "[ui-pass]create-mesh",
            LabelColor::COLOR_STAGE,
        );
        self.meshes[*frame_label].replace(GuiMesh::from_draw_data(rhi, cmd, render_ctx, draw_data));
        rhi.device().debug_utils().end_queue_label(rhi.graphics_queue.handle());

        Some((
            self.meshes[*frame_label].as_ref().unwrap(), //
            draw_data,
        ))
    }

    /// 根据 imgui 传来的 texture id，找到对应的 texture key，用于在 bindless manager 中得到 texture
    pub fn get_texture_key(texture_id: imgui::TextureId) -> String {
        format!("imgui-texture-{}", texture_id.id())
    }
}
