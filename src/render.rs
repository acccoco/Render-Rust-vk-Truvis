use std::ffi::CStr;

use ash::vk;

use crate::{
    render_context::{RenderContext, RenderContextInitInfo},
    rhi::{init_info::RhiInitInfo, Rhi},
    swapchain::{RenderSwapchain, RenderSwapchainInitInfo},
    window_system::{WindowCreateInfo, WindowSystem},
};

pub struct Render;


static mut ENGINE: Option<Render> = None;


pub struct EngineInitInfo
{
    pub window_width: u32,
    pub window_height: u32,
    pub app_name: String,
}

impl Render
{
    const ENGINE_NAME: &'static str = "Hiss";

    #[inline]
    pub fn instance() -> &'static Self { unsafe { ENGINE.as_ref().unwrap() } }

    pub fn init(init_info: &EngineInitInfo)
    {
        simple_logger::SimpleLogger::new().init().unwrap();

        WindowSystem::init(WindowCreateInfo {
            height: init_info.window_height as i32,
            width: init_info.window_width as i32,
            title: init_info.app_name.clone(),
        });

        {
            let mut rhi_init_info = RhiInitInfo::init_basic(Some(vk_debug_callback));
            rhi_init_info.app_name = Some(init_info.app_name.clone());
            rhi_init_info.engine_name = Some(Self::ENGINE_NAME.to_string());
            rhi_init_info.is_complete().unwrap();

            Rhi::init(rhi_init_info);
        }

        RenderSwapchain::init(&RenderSwapchainInitInfo::default());

        RenderContext::init(&RenderContextInitInfo::default());
    }
}

unsafe extern "system" fn vk_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32
{
    let callback_data = *p_callback_data;

    let msg = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    let format_msg = format!("[{:?}] {}", message_type, msg);

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("{}", format_msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("{}", format_msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("{}", format_msg);
        }
        _ => log::info!("{}", format_msg),
    };

    // 只有 layer developer 才需要返回 True
    vk::FALSE
}
