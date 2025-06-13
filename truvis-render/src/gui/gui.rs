//! 参考 imgui-rs-vulkan-renderer

use crate::gui::mesh::GuiMesh;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::FrameContext;
use crate::renderer::pipeline_settings::RendererSettings;
use ash::vk;
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

    /// 3D 渲染的区域
    render_region: vk::Rect2D,

    /// 存放多帧 imgui 的 mesh 数据
    meshes: Vec<Option<GuiMesh>>,

    _device: Rc<RhiDevice>,
}
impl Drop for Gui {
    fn drop(&mut self) {}
}
// region ctor
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

            render_region: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: renderer_settings.frame_settings.rt_extent.width,
                    height: renderer_settings.frame_settings.rt_extent.height,
                },
            },

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
        let io = imgui_ctx.io_mut();
        io.font_global_scale = (1.0 / hidpi_factor) as f32;
        io.config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;

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
// endregion
// region 一般的
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
        let ui = self.imgui_ctx.new_frame();

        unsafe {
            let viewport = imgui::sys::igGetMainViewport();
            let viewport_size = (*viewport).Size;
            let root_node_id = imgui::sys::igGetID_Str(c"MainDockSpace".as_ptr());

            let window_flags = imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::MENU_BAR
                | imgui::WindowFlags::NO_COLLAPSE
                | imgui::WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                | imgui::WindowFlags::NO_NAV_FOCUS
                | imgui::WindowFlags::NO_DOCKING
                | imgui::WindowFlags::NO_BACKGROUND
                | imgui::WindowFlags::NO_RESIZE;

            ui.window("main dock space")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([viewport_size.x, viewport_size.y], imgui::Condition::Always)
                .flags(window_flags)
                .build(|| {
                    if imgui::sys::igDockBuilderGetNode(root_node_id).is_null() {
                        imgui::sys::igDockBuilderRemoveNode(root_node_id);
                        imgui::sys::igDockBuilderAddNode(root_node_id, imgui::sys::ImGuiDockNodeFlags_NoCloseButton);
                        imgui::sys::igDockBuilderSetNodeSize(root_node_id, (*imgui::sys::igGetMainViewport()).Size);
                        // imgui::sys::igDockBuilderSetNodePos(root_id, imgui::sys::ImVec2 { x: 0.0, y: 0.0 });
                        // let root_node = imgui::sys::igDockBuilderGetNode(root_id);
                        // (*root_node).LocalFlags |= imgui::sys::ImGuiDockNodeFlags_HiddenTabBar;

                        let mut dock_main_id = root_node_id;
                        let dock_right_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Right,
                            0.2,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );
                        let dock_left_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Left,
                            0.2,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );
                        let dock_down_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Down,
                            0.2,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );

                        log::info!("main node id: {}", dock_main_id);
                        imgui::sys::igDockBuilderDockWindow(c"left".as_ptr(), dock_left_id);
                        imgui::sys::igDockBuilderDockWindow(c"right".as_ptr(), dock_right_id);
                        imgui::sys::igDockBuilderDockWindow(c"down".as_ptr(), dock_down_id);
                        imgui::sys::igDockBuilderDockWindow(c"render".as_ptr(), dock_main_id);
                        imgui::sys::igDockBuilderFinish(root_node_id);
                    }

                    imgui::sys::igDockSpace(
                        root_node_id,
                        imgui::sys::ImVec2 { x: 0.0, y: 0.0 },
                        imgui::sys::ImGuiDockNodeFlags_None as _,
                        std::ptr::null(),
                    );
                });

            ui.window("left")
                // .size([100.0, 100.0], imgui::Condition::Always)
                // .movable(false)
                // .resizable(false)
                .draw_background(false)
                .title_bar(false)
                .menu_bar(false)
                .build(|| {
                    ui.text_wrapped("Hello world!");
                    ui.text_wrapped("こんにちは世界！");
                    ui.button("This...is...imgui-rs!");
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(format!("Mouse Position: ({:.1},{:.1})", mouse_pos[0], mouse_pos[1]));
                });
            let hidpi_factor = self.platform.hidpi_factor() as f32;
            ui.window("render")
                .title_bar(false)
                .menu_bar(false)
                // .resizable(false)
                // .bg_alpha(0.0)
                .draw_background(false)
                .build(|| {
                    ui.text("render window");
                    // imgui::Image::new(TextureId::new(114), [400.0, 400.0]).build(ui);

                    let window_size = ui.window_size();
                    let window_pos = ui.window_pos();
                    ui.text(format!("Window Size: ({:.1},{:.1})", window_size[0], window_size[1]));
                    ui.text(format!("Window Position: ({:.1},{:.1})", window_pos[0], window_pos[1]));

                    self.render_region.offset = vk::Offset2D {
                        x: (window_pos[0] * hidpi_factor) as i32,
                        y: (window_pos[1] * hidpi_factor) as i32,
                    };
                    self.render_region.extent = vk::Extent2D {
                        width: (window_size[0] * hidpi_factor) as u32,
                        height: (window_size[1] * hidpi_factor) as u32,
                    };
                });
            ui.window("right").draw_background(false).build(|| {
                ui.text("test window.");
                let root_node = imgui::sys::igDockBuilderGetNode(root_node_id);
                let root_pos = (*root_node).Pos;
                let root_size = (*root_node).Size;
                ui.text(format!("Root Node Position: ({:.1},{:.1})", root_pos.x, root_pos.y));
                ui.text(format!("Root Node Size: ({:.1},{:.1})", root_size.x, root_size.y));
            });
            ui.window("down").build(|| {
                ui.text("down window");
                ui.text("This is a test window.");
                ui.text("You can put anything you want here.");
            });
        }

        ui_func(ui);
        // 看源码可知：imgui 可能会隐藏鼠标指针
        self.platform.prepare_render(ui, window);
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

    #[inline]
    pub fn get_render_region(&self) -> vk::Rect2D {
        self.render_region
    }
}
// endregion
