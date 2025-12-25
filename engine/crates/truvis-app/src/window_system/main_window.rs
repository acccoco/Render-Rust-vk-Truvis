use ash::vk;
use winit::window::Window;

use truvis_render_core::platform::event::InputEvent;
use truvis_render_core::present::gui_front::GuiHost;

mod helper {}

pub struct MainWindow {
    winit_window: Window,
    pub gui_host: GuiHost,
}
// new & init
impl MainWindow {}
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
    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        if let winit::event::Event::WindowEvent { event, .. } = event {
            let input_event = InputEvent::from_winit_event(event);
            self.gui_host.handle_event(&input_event);
        }
    }

    /// imgui 中用于绘制图形的区域大小
    pub fn get_render_extent(&self) -> vk::Extent2D {
        self.gui_host.get_render_region().extent
    }
}
