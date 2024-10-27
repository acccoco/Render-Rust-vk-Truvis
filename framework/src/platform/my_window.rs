use ash::{extensions::khr::Surface, vk::SurfaceKHR};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::{
    core::instance::Instance,
    platform::window::{Window, WindowProperties},
};

pub struct MyWindow
{
    window_properties: WindowProperties,

    handle: winit::window::Window,
}

impl MyWindow
{
    pub fn new(properties: WindowProperties) -> anyhow::Result<Self>
    {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::WindowBuilder::new()
            .with_title(properties.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                properties.extent.width,
                properties.extent.height,
            ))
            .build(&event_loop)?;

        Ok(Self {
            window_properties: properties,
            handle: window,
        })
    }
}

impl Window for MyWindow
{
    fn get_properties(&self) -> &crate::platform::window::WindowProperties
    {
        &self.window_properties
    }

    fn get_properties_mut(&mut self) -> &mut crate::platform::window::WindowProperties
    {
        &mut self.window_properties
    }

    fn close(&self)
    {
        todo!()
    }

    fn create_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> anyhow::Result<SurfaceKHR>
    {
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                self.handle.raw_display_handle(),
                self.handle.raw_window_handle(),
                None,
            )
        }?;
        Ok(surface)
    }

    fn should_close(&self) -> bool
    {
        todo!()
    }

    fn process_events(&self) {
        todo!()
    }

    fn get_required_surface_extensions(&self) -> Vec<String> {
        todo!()
    }
}
