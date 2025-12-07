use crate::foundation::device::GfxDevice;
use ash::vk;
use std::collections::HashMap;
use std::rc::Rc;

// TODO Sampler manager 应该放到 renderer 层级，而不是 GFX 层级
// Sampler descriptor
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GfxSamplerDesc {
    pub mag_filter: vk::Filter,
    pub min_filter: vk::Filter,
    pub address_mode_u: vk::SamplerAddressMode,
    pub address_mode_v: vk::SamplerAddressMode,
    pub address_mode_w: vk::SamplerAddressMode,
    pub max_anisotropy: u32,
    pub compare_op: Option<vk::CompareOp>,
    pub mipmap_mode: vk::SamplerMipmapMode,
}

impl Default for GfxSamplerDesc {
    fn default() -> Self {
        Self {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            max_anisotropy: 0,
            compare_op: None,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
        }
    }
}

// Sampler manager
pub struct GfxSamplerManager {
    samplers: HashMap<GfxSamplerDesc, vk::Sampler>, // deduplicate identical sampler

    device: Rc<GfxDevice>,
}

impl GfxSamplerManager {
    pub fn new(device: Rc<GfxDevice>) -> Self {
        Self {
            samplers: HashMap::new(),
            device,
        }
    }

    pub fn get_sampler(&mut self, desc: &GfxSamplerDesc) -> vk::Sampler {
        if let Some(&sampler) = self.samplers.get(desc) {
            sampler
        } else {
            let vk_sampler = Self::create_vk_sampler(&self.device, desc);
            self.samplers.insert(*desc, vk_sampler);
            vk_sampler
        }
    }

    pub fn destroy(&mut self) {
        for (_, sampler) in self.samplers.drain() {
            unsafe {
                self.device.destroy_sampler(sampler, None);
            }
        }
    }

    fn create_vk_sampler(device: &GfxDevice, desc: &GfxSamplerDesc) -> vk::Sampler {
        let mut create_info = vk::SamplerCreateInfo::default()
            .mag_filter(desc.mag_filter)
            .min_filter(desc.min_filter)
            .address_mode_u(desc.address_mode_u)
            .address_mode_v(desc.address_mode_v)
            .address_mode_w(desc.address_mode_w)
            .mipmap_mode(desc.mipmap_mode)
            .min_lod(0.0)
            .max_lod(vk::LOD_CLAMP_NONE)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK);

        if desc.max_anisotropy > 0 {
            create_info = create_info.anisotropy_enable(true).max_anisotropy(desc.max_anisotropy as f32);
        } else {
            create_info = create_info.anisotropy_enable(false);
        }

        if let Some(compare_op) = desc.compare_op {
            create_info = create_info.compare_enable(true).compare_op(compare_op);
        } else {
            create_info = create_info.compare_enable(false);
        }

        unsafe { device.create_sampler(&create_info, None).expect("Failed to create sampler") }
    }
}
