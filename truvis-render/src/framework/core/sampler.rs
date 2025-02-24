use ash::vk;

use crate::framework::render_core::Core;

pub struct Sampler
{
    min_filter: vk::Filter,
    mag_filter: vk::Filter,
    mipmap_mode: vk::SamplerMipmapMode,
    wrap_u: vk::SamplerAddressMode,
    wrap_v: vk::SamplerAddressMode,

    handle: vk::Sampler,
    rhi: &'static Core,
}

impl Sampler
{
    pub fn new(rhi: &'static Core, info: &vk::SamplerCreateInfo, debug_name: &str) -> Self
    {
        let handle = unsafe { rhi.vk_device().create_sampler(info, None).unwrap() };
        rhi.set_debug_name(handle, debug_name);

        Self {
            min_filter: info.min_filter,
            mag_filter: info.mag_filter,
            mipmap_mode: info.mipmap_mode,
            wrap_u: info.address_mode_u,
            wrap_v: info.address_mode_v,
            handle,
            rhi,
        }
    }
}
