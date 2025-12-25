use crate::foundation::debug_messenger::DebugType;
use crate::gfx_core::GfxCore;
use ash::vk;

pub struct GfxSurface {
    pub(crate) handle: vk::SurfaceKHR,
    pub(crate) pf: ash::khr::surface::Instance,

    pub(crate) capabilities: vk::SurfaceCapabilitiesKHR,
}

impl GfxSurface {
    pub fn new(
        vk_core: &GfxCore,
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
    ) -> Self {
        let surface_pf = ash::khr::surface::Instance::new(&vk_core.vk_entry, &vk_core.instance.ash_instance);

        let surface = unsafe {
            ash_window::create_surface(
                &vk_core.vk_entry,
                &vk_core.instance.ash_instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )
            .unwrap()
        };

        let surface_capabilities = unsafe {
            surface_pf.get_physical_device_surface_capabilities(vk_core.physical_device.vk_handle, surface).unwrap()
        };

        let surface = GfxSurface {
            handle: surface,
            pf: surface_pf,
            capabilities: surface_capabilities,
        };
        vk_core.gfx_device.set_debug_name(&surface, "main");

        surface
    }
}

impl Drop for GfxSurface {
    fn drop(&mut self) {
        unsafe { self.pf.destroy_surface(self.handle, None) }
    }
}

impl DebugType for GfxSurface {
    fn debug_type_name() -> &'static str {
        "GfxSurface"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
