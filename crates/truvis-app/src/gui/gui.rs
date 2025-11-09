//! 参考 imgui-rs-vulkan-renderer

use std::rc::Rc;

use ash::vk;

use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::swapchain::render_swapchain::SwapchainImageInfo;
use truvis_gfx::{
    basic::color::LabelColor,
    commands::command_buffer::CommandBuffer,
    gfx::Gfx,
    resources::{image::Image2D, texture::Texture2D},
};
use truvis_render::pipeline_settings::FrameLabel;
use truvis_render::renderer::bindless::BindlessManager;
use truvis_render::renderer::frame_context::FrameContext;

use crate::gui::gui_mesh::GuiMesh;

pub struct Gui {
    pub imgui_ctx: imgui::Context,
    pub platform: imgui_winit_support::WinitPlatform,

    /// 3D 渲染的区域
    render_region: vk::Rect2D,

    /// 存放多帧 imgui 的 mesh 数据
    meshes: Vec<Option<GuiMesh>>,
    render_image_key: Option<String>,
}
// 创建过程
impl Gui {
    const FONT_TEXTURE_ID: usize = 0;
    const FONT_TEXTURE_KEY: &'static str = "imgui-fonts";
    const RENDER_IMAGE_ID: usize = 1;

    pub fn new(window: &winit::window::Window, fif_num: usize, swapchain_image_infos: &SwapchainImageInfo) -> Self {
        let mut imgui_ctx = imgui::Context::create();
        // disable automatic saving .ini file
        imgui_ctx.set_ini_filename(None);

        // theme
        {
            let style = imgui_ctx.style_mut();
            style.use_dark_colors();
            style.colors[imgui::StyleColor::WindowBg as usize] = [1.0, 0.0, 0.0, 1.0];
        }

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui_ctx);
        platform.attach_window(imgui_ctx.io_mut(), window, imgui_winit_support::HiDpiMode::Rounded);

        let mut bindless_mgr = FrameContext::bindless_mgr_mut();
        Self::init_fonts(&mut imgui_ctx, &platform, &mut bindless_mgr);

        Self {
            imgui_ctx,
            platform,

            render_region: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_image_infos.image_extent,
            },

            meshes: (0..fif_num).map(|_| None).collect(),
            render_image_key: None,
        }
    }

    /// 初始化的时候注册字体
    ///
    /// 1. 首先将字体数据放入 imgui 中，并建立起字体 atlas
    /// 1. 然后将字体 atlas 转换为 GfxImage2D，并注册到 BindlessManager 中
    ///
    /// # Return
    /// ```
    ///     "font texture id in imgui"
    /// ```
    fn init_fonts(
        imgui_ctx: &mut imgui::Context,
        platform: &imgui_winit_support::WinitPlatform,
        bindless_mgr: &mut BindlessManager,
    ) {
        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;

        let font_data = std::fs::read(TruvisPath::resources_path("mplus-1p-regular.ttf")).unwrap();
        imgui_ctx.fonts().add_font(&[
            imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    size_pixels: font_size,
                    ..Default::default()
                }),
            },
            imgui::FontSource::TtfData {
                data: font_data.as_ref(),
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

            let image = Rc::new(Image2D::from_rgba8(
                atlas_texture.width,
                atlas_texture.height,
                atlas_texture.data,
                "imgui-fonts",
            ));
            Texture2D::new(image, "imgui-fonts")
        };

        let fonts_texture_id = imgui::TextureId::from(Self::FONT_TEXTURE_ID);
        bindless_mgr.register_texture_owned(Self::FONT_TEXTURE_KEY.to_string(), fonts_texture);
        imgui_ctx.fonts().tex_id = fonts_texture_id;
    }
}
// tools
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
    pub fn update(
        &mut self,
        window: &winit::window::Window,
        ui_func_main: impl FnOnce(&imgui::Ui, [f32; 2]),
        ui_func_right: impl FnOnce(&imgui::Ui),
    ) {
        let ui = self.imgui_ctx.new_frame();

        unsafe {
            let viewport = imgui::sys::igGetMainViewport();
            let viewport_size = (*viewport).Size;
            let root_node_id = imgui::sys::igGetID_Str(c"MainDockSpace".as_ptr());

            ui.window("main dock space")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([viewport_size.x, viewport_size.y], imgui::Condition::Always)
                .flags(
                    imgui::WindowFlags::NO_MOVE
                    | imgui::WindowFlags::NO_TITLE_BAR
                    // | imgui::WindowFlags::MENU_BAR
                    | imgui::WindowFlags::NO_COLLAPSE
                    | imgui::WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                    | imgui::WindowFlags::NO_NAV_FOCUS
                    | imgui::WindowFlags::NO_DOCKING
                    | imgui::WindowFlags::NO_BACKGROUND
                    | imgui::WindowFlags::NO_RESIZE,
                )
                .build(|| {
                    if imgui::sys::igDockBuilderGetNode(root_node_id).is_null() {
                        imgui::sys::igDockBuilderRemoveNode(root_node_id);
                        imgui::sys::igDockBuilderAddNode(root_node_id, imgui::sys::ImGuiDockNodeFlags_NoCloseButton);
                        imgui::sys::igDockBuilderSetNodeSize(root_node_id, (*imgui::sys::igGetMainViewport()).Size);
                        imgui::sys::igDockBuilderSetNodePos(root_node_id, imgui::sys::ImVec2 { x: 0.0, y: 0.0 });

                        // 首先将整个窗口分为左右两部分
                        let mut dock_main_id = root_node_id;
                        let dock_right_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Right,
                            0.3,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );

                        // 将左边部分再分为左右两部分
                        // let dock_left_id = imgui::sys::igDockBuilderSplitNode(
                        //     dock_main_id,
                        //     imgui::sys::ImGuiDir_Left,
                        //     0.2,
                        //     std::ptr::null_mut(),
                        //     std::ptr::from_mut(&mut dock_main_id),
                        // );

                        // 将中间部分在分为上下两部分
                        // let dock_down_id = imgui::sys::igDockBuilderSplitNode(
                        //     dock_main_id,
                        //     imgui::sys::ImGuiDir_Down,
                        //     0.2,
                        //     std::ptr::null_mut(),
                        //     std::ptr::from_mut(&mut dock_main_id),
                        // );

                        // 隐藏中央节点的 Tab
                        // let center_node = imgui::sys::igDockBuilderGetNode(dock_main_id);
                        // (*center_node).LocalFlags |= imgui::sys::ImGuiDockNodeFlags_HiddenTabBar;

                        log::info!("main node id: {}", dock_main_id);
                        imgui::sys::igDockBuilderDockWindow(c"right".as_ptr(), dock_right_id);
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

            // 中间的窗口，用于放置渲染内容
            ui.window("render")
                .title_bar(true)
                .menu_bar(false)
                // .resizable(false)
                // .bg_alpha(0.0)
                .draw_background(false)
                .build(|| {
                    let window_pos = ui.window_pos();
                    let window_region_max = ui.window_content_region_max();
                    let window_region_min = ui.window_content_region_min();
                    let window_size = [
                        window_region_max[0] - window_region_min[0],
                        window_region_max[1] - window_region_min[1],
                    ];
                    // let hidpi_factor = self.platform.hidpi_factor() as f32;
                    let hidpi_factor = 1.0;

                    self.render_region.offset = vk::Offset2D {
                        x: (window_pos[0] * hidpi_factor) as i32,
                        y: (window_pos[1] * hidpi_factor) as i32,
                    };
                    self.render_region.extent = vk::Extent2D {
                        width: (window_size[0] * hidpi_factor) as u32,
                        height: (window_size[1] * hidpi_factor) as u32,
                    };

                    imgui::Image::new(imgui::TextureId::new(Self::RENDER_IMAGE_ID), [window_size[0], window_size[1]])
                        .build(ui);

                    ui_func_main(ui, window_size);
                });

            // 右侧的窗口，用于放置各种设置
            ui.window("right").draw_background(false).build(|| {
                ui.text("test window.");
                let root_node = imgui::sys::igDockBuilderGetNode(root_node_id);
                let root_pos = (*root_node).Pos;
                let root_size = (*root_node).Size;
                ui.text(format!("Root Node Position: ({:.1},{:.1})", root_pos.x, root_pos.y));
                ui.text(format!("Root Node Size: ({:.1},{:.1})", root_size.x, root_size.y));

                let hidpi_factor = self.platform.hidpi_factor();
                ui.text(format!("Hidpi Factor: {}", hidpi_factor));
                ui.text(format!("Window Size: ({:?})", self.render_region.extent));
                ui.text(format!("Window Position: ({:?})", self.render_region.offset));
                ui.new_line();

                ui_func_right(ui);
            });
        }

        // 看源码可知：imgui 可能会隐藏鼠标指针
        self.platform.prepare_render(ui, window);
    }

    pub fn register_render_image_key(&mut self, key: String) {
        self.render_image_key = Some(key);
    }

    /// # Phase: Render
    ///
    /// 使用 imgui 将 ui 操作编译为 draw data；构建 draw 需要的 mesh 数据
    pub fn imgui_render(
        &mut self,

        cmd: &CommandBuffer,
        frame_label: FrameLabel,
    ) -> Option<(&GuiMesh, &imgui::DrawData, impl Fn(imgui::TextureId) -> String + use<'_>)> {
        let draw_data = self.imgui_ctx.render();
        if draw_data.total_vtx_count == 0 {
            return None;
        }

        Gfx::get().gfx_queue().begin_label("[ui-pass]create-mesh", LabelColor::COLOR_STAGE);
        self.meshes[*frame_label].replace(GuiMesh::new(cmd, &format!("{frame_label}"), draw_data));
        Gfx::get().gfx_queue().end_label();

        Some((
            self.meshes[*frame_label].as_ref().unwrap(), //
            draw_data,
            |texture_id: imgui::TextureId| match texture_id.id() {
                Self::RENDER_IMAGE_ID => self.render_image_key.as_ref().unwrap().clone(),
                Self::FONT_TEXTURE_ID => Self::FONT_TEXTURE_KEY.to_string(),
                _ => format!("imgui-texture-{}", texture_id.id()),
            },
        ))
    }

    #[inline]
    pub fn get_render_region(&self) -> vk::Rect2D {
        self.render_region
    }
}
impl Drop for Gui {
    fn drop(&mut self) {
        // 每个字段都是 RAII 的
    }
}
