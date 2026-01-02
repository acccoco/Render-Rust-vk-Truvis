use crate::foundation::debug_messenger::DebugType;
use crate::gfx::Gfx;
use ash::vk;

pub struct GfxSurface {
    pub(crate) handle: vk::SurfaceKHR,
    pub(crate) pf: ash::khr::surface::Instance,

    pub(crate) capabilities: vk::SurfaceCapabilitiesKHR,
}

impl GfxSurface {
    pub fn new(
        raw_display_handle: raw_window_handle::RawDisplayHandle,
        raw_window_handle: raw_window_handle::RawWindowHandle,
    ) -> Self {
        let gfx_core = &Gfx::get().gfx_core;
        let surface_pf = ash::khr::surface::Instance::new(&gfx_core.vk_entry, &gfx_core.instance.ash_instance);

        let surface = unsafe {
            ash_window::create_surface(
                &gfx_core.vk_entry,
                &gfx_core.instance.ash_instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )
            .unwrap()
        };

        let surface_capabilities = unsafe {
            surface_pf.get_physical_device_surface_capabilities(gfx_core.physical_device.vk_handle, surface).unwrap()
        };

        let surface = GfxSurface {
            handle: surface,
            pf: surface_pf,
            capabilities: surface_capabilities,
        };
        gfx_core.gfx_device.set_debug_name(&surface, "main");

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
