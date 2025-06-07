use std::rc::Rc;

use crate::core::debug_utils::RhiDebugType;
use crate::core::device::RhiDevice;
use crate::rhi::Rhi;
use ash::vk;

pub struct RhiSamplerCreateInfo {
    inner: vk::SamplerCreateInfo<'static>,
}

impl Default for RhiSamplerCreateInfo {
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

impl RhiSamplerCreateInfo {
    /// 默认配置：linear，repeat
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct RhiSampler {
    handle: vk::Sampler,

    _info: Rc<RhiSamplerCreateInfo>,
    device: Rc<RhiDevice>,
}
impl RhiDebugType for RhiSampler {
    fn debug_type_name() -> &'static str {
        "RhiSampler"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
impl Drop for RhiSampler {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.handle, None);
        }
    }
}

impl RhiSampler {
    #[inline]
    pub fn new(rhi: &Rhi, info: Rc<RhiSamplerCreateInfo>, debug_name: &str) -> Self {
        let handle = unsafe { rhi.device.create_sampler(&info.inner, None).unwrap() };
        let sampler = Self {
            handle,
            _info: info,
            device: rhi.device.clone(),
        };
        rhi.device.debug_utils().set_debug_name(&sampler, debug_name);
        sampler
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::Sampler {
        self.handle
    }
}
