use std::rc::Rc;

use ash::vk;

use crate::framework::render_core::Rhi;

pub struct RhiSamplerCreateInfo
{
    inner: vk::SamplerCreateInfo<'static>,
}

impl RhiSamplerCreateInfo
{
    /// 默认配置：linear，repeat
    #[inline]
    pub fn new() -> Self
    {
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

pub struct RhiSampler
{
    handle: vk::Sampler,

    info: Rc<RhiSamplerCreateInfo>,
}

impl RhiSampler
{
    #[inline]
    pub fn new(rhi: &Rhi, info: Rc<RhiSamplerCreateInfo>, debug_name: &str) -> Self
    {
        let handle = unsafe { rhi.device.create_sampler(&info.inner, None).unwrap() };
        rhi.set_debug_name(handle, debug_name);

        Self { handle, info }
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::Sampler
    {
        self.handle
    }
}
