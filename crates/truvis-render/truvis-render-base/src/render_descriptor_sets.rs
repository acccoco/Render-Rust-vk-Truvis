use crate::frame_counter::FrameCounter;
use crate::pipeline_settings::FrameLabel;
use ash::vk;
use itertools::Itertools;
use std::rc::Rc;
use truvis_gfx::descriptors::descriptor::{GfxDescriptorSet, GfxDescriptorSetLayout};
use truvis_gfx::descriptors::descriptor_pool::{GfxDescriptorPool, GfxDescriptorPoolCreateInfo};
use truvis_shader_layout_macro::DescriptorBinding;

#[derive(DescriptorBinding)]
pub struct BindlessDescriptorBinding {
    #[binding = 0]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _textures: (),

    #[binding = 1]
    #[descriptor_type = "STORAGE_IMAGE"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _uavs: (),

    #[binding = 2]
    #[descriptor_type = "SAMPLED_IMAGE"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 128]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _srvs: (),
}

#[derive(DescriptorBinding)]
pub struct GlobalDescriptorBinding {
    #[binding = 0]
    #[descriptor_type = "SAMPLER"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 32]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _samplers: (),
}

pub struct RenderDescriptorSets {
    pub layout_0_global: GfxDescriptorSetLayout<GlobalDescriptorBinding>,
    pub set_0_global: GfxDescriptorSet<GlobalDescriptorBinding>,

    pub layout_1_bindless: GfxDescriptorSetLayout<BindlessDescriptorBinding>,
    pub set_1_bindless: [GfxDescriptorSet<BindlessDescriptorBinding>; FrameCounter::fif_count()],

    _descriptor_pool: GfxDescriptorPool,
}
// new & init
impl RenderDescriptorSets {
    pub fn new() -> Self {
        let descriptor_pool = Self::init_descriptor_pool();

        let bindless_layout = GfxDescriptorSetLayout::<BindlessDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "bindless-layout",
        );
        let bindless_descriptor_sets = FrameCounter::frame_labes().map(|frame_label| {
            GfxDescriptorSet::<BindlessDescriptorBinding>::new(
                &descriptor_pool,
                &bindless_layout,
                format!("bindless-descriptor-set-{frame_label}"),
            )
        });

        let global_layout = GfxDescriptorSetLayout::<GlobalDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            "global-layout",
        );
        let global_descriptor_set =
            GfxDescriptorSet::<GlobalDescriptorBinding>::new(&descriptor_pool, &global_layout, "global-descriptor-set");

        Self {
            layout_0_global: global_layout,
            set_0_global: global_descriptor_set,

            layout_1_bindless: bindless_layout,
            set_1_bindless: bindless_descriptor_sets,

            _descriptor_pool: descriptor_pool,
        }
    }

    fn init_descriptor_pool() -> GfxDescriptorPool {
        let pool_size = [
            (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, 512),
            (vk::DescriptorType::STORAGE_IMAGE, 512),
            (vk::DescriptorType::SAMPLED_IMAGE, 512),
            (vk::DescriptorType::SAMPLER, 32),
        ]
        .iter()
        .map(|(ty, count)| vk::DescriptorPoolSize {
            ty: *ty,
            descriptor_count: *count,
        })
        .collect_vec();

        let pool_ci = Rc::new(GfxDescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            32,
            pool_size,
        ));

        GfxDescriptorPool::new(pool_ci, "renderer")
    }
}
impl Default for RenderDescriptorSets {
    fn default() -> Self {
        Self::new()
    }
}
// destroy
impl RenderDescriptorSets {
    pub fn destroy_mut(&mut self) {
        // descriptor sets 跟随 pool 一起销毁
    }
    pub fn destroy(self) {
        // descriptor sets 跟随 pool 一起销毁
    }
}
impl Drop for RenderDescriptorSets {
    fn drop(&mut self) {
        self.destroy_mut();
    }
}
// getters
impl RenderDescriptorSets {
    #[inline]
    pub fn current_bindless_descriptor_set(
        &self,
        frame_label: FrameLabel,
    ) -> &GfxDescriptorSet<BindlessDescriptorBinding> {
        &self.set_1_bindless[*frame_label]
    }
}
