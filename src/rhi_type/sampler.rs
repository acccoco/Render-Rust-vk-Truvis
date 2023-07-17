use ash::vk;

use crate::rhi::Rhi;

pub struct RhiSampler
{
    min_filter: vk::Filter,
    mag_filter: vk::Filter,
    mipmap_mode: vk::SamplerMipmapMode,
    wrap_u: vk::SamplerAddressMode,
    wrap_v: vk::SamplerAddressMode,

    handle: vk::Sampler,
}

impl RhiSampler
{
    pub fn new(info: &vk::SamplerCreateInfo, debug_name: &str) -> Self
    {
        let handle = unsafe { Rhi::instance().device().create_sampler(info, None).unwrap() };
        Rhi::instance().set_debug_name(handle, debug_name);

        Self {
            min_filter: info.min_filter,
            mag_filter: info.mag_filter,
            mipmap_mode: info.mipmap_mode,
            wrap_u: info.address_mode_u,
            wrap_v: info.address_mode_v,
            handle,
        }
    }
}
