use ash::{extensions::khr::Surface, vk, vk::SurfaceKHR};

use crate::core::instance::Instance;

#[derive(Copy, Clone)]
pub enum WindowMode
{
    Headless,
    Fullscreen,
    FullscreenBorderless,
    FullscreenStretch,
    Default,
}


#[derive(Clone)]
pub struct WindowProperties
{
    pub title: String,
    pub mode: WindowMode,
    pub resizable: bool,
    pub extent: vk::Extent2D,
}

impl Default for WindowProperties
{
    fn default() -> Self
    {
        Self {
            title: "".to_string(),
            mode: WindowMode::Default,
            resizable: true,
            extent: vk::Extent2D {
                width: 1280,
                height: 720,
            },
        }
    }
}

pub trait Window
{
    fn get_properties(&self) -> &WindowProperties;
    fn get_properties_mut(&mut self) -> &mut WindowProperties;

    fn close(&self);

    fn create_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> anyhow::Result<SurfaceKHR>;

    fn should_close(&self) -> bool;

    /// 尝试修改窗口尺寸，不保证能够修改成功
    ///
    /// return: 修改后的窗口尺寸
    fn resize(&mut self, new_extent: vk::Extent2D) -> vk::Extent2D
    {
        let mut properties = self.get_properties_mut();
        if properties.resizable {
            properties.extent = new_extent;
        }
        properties.extent
    }

    fn get_extent(&self) -> vk::Extent2D
    {
        self.get_properties().extent
    }

    fn get_window_mode(&self) -> WindowMode
    {
        self.get_properties().mode
    }

    fn process_events(&self);

    fn get_required_surface_extensions(&self) -> Vec<String>;
}
