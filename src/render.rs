use crate::{
    render_context::{RenderContext, RenderContextInitInfo},
    rhi::{
        rhi_init_info::{vk_debug_callback, RhiInitInfo},
        Rhi,
    },
    swapchain::{RenderSwapchain, RenderSwapchainInitInfo},
    window_system::{WindowCreateInfo, WindowSystem},
};

pub struct Renderer;

use anyhow::Context;

static mut RENDERER: Option<Renderer> = None;


pub fn panic_handler(info: &std::panic::PanicInfo)
{
    log::error!("{}", info);
    std::thread::sleep(std::time::Duration::from_secs(3));
}


pub struct RenderInitInfo
{
    pub window_width: u32,
    pub window_height: u32,
    pub app_name: String,
}

impl Renderer
{
    const ENGINE_NAME: &'static str = "Hiss";

    #[inline]
    pub fn instance() -> &'static Self
    {
        unsafe { RENDERER.as_ref().unwrap() }
    }

    pub fn init(init_info: &RenderInitInfo) -> anyhow::Result<()>
    {
        simple_logger::SimpleLogger::new().init()?;

        std::panic::set_hook(Box::new(panic_handler));

        WindowSystem::init(WindowCreateInfo {
            height: init_info.window_height as i32,
            width: init_info.window_width as i32,
            title: init_info.app_name.clone(),
        });

        {
            let mut rhi_init_info = RhiInitInfo::init_basic(Some(vk_debug_callback));
            rhi_init_info.app_name = Some(init_info.app_name.clone());
            rhi_init_info.engine_name = Some(Self::ENGINE_NAME.to_string());
            rhi_init_info.is_complete()?;

            Rhi::init(rhi_init_info)?;
        }

        RenderSwapchain::init(&RenderSwapchainInitInfo::default());

        RenderContext::init(&RenderContextInitInfo::default());

        Ok(())
    }
}
