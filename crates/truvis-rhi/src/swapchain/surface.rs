use crate::foundation::debug_messenger::DebugType;
use crate::vulkan_core::VulkanCore;
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct Surface {
    pub(crate) handle: vk::SurfaceKHR,
    pub(crate) pf: ash::khr::surface::Instance,

    pub(crate) capabilities: vk::SurfaceCapabilitiesKHR,
}

impl Surface {
    pub fn new(vk_core: &VulkanCore, window: &winit::window::Window) -> Self {
        let surface_pf = ash::khr::surface::Instance::new(&vk_core.vk_pf, &vk_core.instance.ash_instance);

        let surface = unsafe {
            ash_window::create_surface(
                &vk_core.vk_pf,
                &vk_core.instance.ash_instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
            .unwrap()
        };

        let surface_capabilities = unsafe {
            surface_pf.get_physical_device_surface_capabilities(vk_core.physical_device.vk_handle, surface).unwrap()
        };

        let surface = Surface {
            handle: surface,
            pf: surface_pf,
            capabilities: surface_capabilities,
        };
        vk_core.device_functions.set_debug_name(&surface, "main");

        surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.pf.destroy_surface(self.handle, None) }
    }
}

impl DebugType for Surface {
    fn debug_type_name() -> &'static str {
        "RhiSurface"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
