use std::rc::Rc;

use ash::vk;

use crate::{foundation::debug_messenger::DebugType, render_context::RenderContext};

pub struct SamplerCreateInfo {
    inner: vk::SamplerCreateInfo<'static>,
}

impl Default for SamplerCreateInfo {
    fn default() -> Self {
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(1.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(1.0);

        Self { inner: sampler_info }
    }
}

impl SamplerCreateInfo {
    /// 默认配置：linear，repeat
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct Sampler {
    handle: vk::Sampler,

    _info: Rc<SamplerCreateInfo>,
}
impl DebugType for Sampler {
    fn debug_type_name() -> &'static str {
        "GfxSampler"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
impl Drop for Sampler {
    fn drop(&mut self) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            device_functions.destroy_sampler(self.handle, None);
        }
    }
}

impl Sampler {
    #[inline]
    pub fn new(info: Rc<SamplerCreateInfo>, debug_name: &str) -> Self {
        let device_functions = RenderContext::get().device_functions();
        let handle = unsafe { device_functions.create_sampler(&info.inner, None).unwrap() };
        let sampler = Self { handle, _info: info };
        device_functions.set_debug_name(&sampler, debug_name);
        sampler
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::Sampler {
        self.handle
    }
}
