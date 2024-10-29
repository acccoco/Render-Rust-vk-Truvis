use anyhow::Context;

use crate::{
    framework::{
        core::swapchain::{RenderSwapchain, RenderSwapchainInitInfo},
        platform::window_system::{WindowCreateInfo, WindowSystem},
        rendering::render_context::{RenderContext, RenderContextInitInfo},
        rhi::{vk_debug_callback, Rhi, RhiInitInfo},
    },
    render_init::ENGINE_NAME,
};


/// 表示整个渲染器进程，需要考虑 platform, render, rhi, log 之类的各种模块
pub struct Renderer;

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
            rhi_init_info.engine_name = Some(ENGINE_NAME.to_string());
            rhi_init_info.is_complete()?;

            Rhi::init(rhi_init_info)?;
        }

        RenderSwapchain::init(&RenderSwapchainInitInfo::default());

        RenderContext::init(&RenderContextInitInfo::default());

        Ok(())
    }
}
