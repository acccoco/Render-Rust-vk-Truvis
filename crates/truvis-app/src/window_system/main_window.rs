use ash::vk;
use itertools::Itertools;
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows, window::Window};

use truvis_crate_tools::resource::TruvisPath;
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_render_core::present::gui_backend::GuiBackend;
use truvis_render_core::present::gui_front::GuiHost;
use truvis_resource::gfx_resource_manager::GfxResourceManager;

mod helper {
    pub fn load_icon(bytes: &[u8]) -> winit::window::Icon {
        let (icon_rgba, icon_width, icon_height) = {
            let image = image::load_from_memory(bytes).unwrap().into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };
        winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
    }
}

pub struct MainWindow {
    winit_window: Window,
    pub gui_host: GuiHost,
}
// new & init
impl MainWindow {
    pub fn new(event_loop: &ActiveEventLoop, window_title: String, window_extent: vk::Extent2D) -> Self {
        let icon_data = std::fs::read(TruvisPath::resources_path("DruvisIII.png")).expect("Failed to read icon file");
        let icon = helper::load_icon(icon_data.as_ref());
        let window_attr = Window::default_attributes()
            .with_title(window_title)
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(window_extent.width as f64, window_extent.height as f64));

        let window = event_loop.create_window(window_attr).unwrap();

        let gui_host = GuiHost::new(&window);

        Self {
            winit_window: window,
            gui_host,
        }
    }

    pub fn init_with_gui_backend(
        &mut self,
        gui_backend: &mut GuiBackend,
        bindless_manager: &mut BindlessManager,
        gfx_resource_manager: &mut GfxResourceManager,
    ) {
        let (fonts_atlas, font_tex_id) = self.gui_host.init_font();
        gui_backend.register_font(bindless_manager, gfx_resource_manager, fonts_atlas, font_tex_id);
    }
}
// destroy
impl MainWindow {
    pub fn destroy(self) {
        // TODO 似乎全都是 RAII 的
    }
}
// getters
impl MainWindow {
    #[inline]
    pub fn window(&self) -> &Window {
        &self.winit_window
    }
}
// tools
impl MainWindow {}
// phase
impl MainWindow {
    pub fn update_gui(&mut self, elapsed: std::time::Duration, ui_func_right: impl FnOnce(&imgui::Ui)) {
        self.gui_host.prepare_frame(&self.winit_window, elapsed);
        self.gui_host.update(
            &self.winit_window,
            |ui, content_size| {
                let min_pos = ui.window_content_region_min();
                ui.set_cursor_pos([min_pos[0] + 5.0, min_pos[1] + 5.0]);
                ui.text(format!("FPS: {:.2}", 1.0 / elapsed.as_secs_f32()));
                ui.text(format!("size: {:.0}x{:.0}", content_size[0], content_size[1]));
            },
            ui_func_right,
        );
    }

    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.gui_host.handle_event(&self.winit_window, event);
    }

    /// imgui 中用于绘制图形的区域大小
    pub fn get_render_extent(&self) -> vk::Extent2D {
        self.gui_host.get_render_region().extent
    }
}
