use crate::frame_counter::FrameCounter;
use crate::pipeline_settings::FrameLabel;
use ash::vk;
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
    _images: (),

    #[binding = 2]
    #[descriptor_type = "SAMPLER"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 32]
    #[flags = "PARTIALLY_BOUND | UPDATE_AFTER_BIND"]
    _samplers: (),
}

#[derive(DescriptorBinding)]
pub struct PerFrameDescriptorBinding {
    #[binding = 0]
    #[descriptor_type = "UNIFORM_BUFFER"]
    #[stage = "FRAGMENT | RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR | COMPUTE"]
    #[count = 1]
    _per_frame_data: (),
}

pub struct RenderDescriptorSets {
    pub layout_0_bindless: GfxDescriptorSetLayout<BindlessDescriptorBinding>,
    pub set_0_bindless: [GfxDescriptorSet<BindlessDescriptorBinding>; FrameCounter::fif_count()],

    pub layout_1_perframe: GfxDescriptorSetLayout<PerFrameDescriptorBinding>,
    pub set_1_perframe: [GfxDescriptorSet<PerFrameDescriptorBinding>; FrameCounter::fif_count()],

    descriptor_pool: GfxDescriptorPool,
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

        let per_frame_layout = GfxDescriptorSetLayout::<PerFrameDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "per-frame-layout",
        );
        let per_frame_descriptor_sets = FrameCounter::frame_labes().map(|frame_label| {
            GfxDescriptorSet::<PerFrameDescriptorBinding>::new(
                &descriptor_pool,
                &per_frame_layout,
                format!("per-frame-descriptor-set-{frame_label}"),
            )
        });

        Self {
            layout_0_bindless: bindless_layout,
            set_0_bindless: bindless_descriptor_sets,
            layout_1_perframe: per_frame_layout,
            set_1_perframe: per_frame_descriptor_sets,
            descriptor_pool,
        }
    }

    const DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_MATERIAL_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT: u32 = 128;

    fn init_descriptor_pool() -> GfxDescriptorPool {
        let pool_size = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT + 32,
            },
        ];

        let pool_ci = Rc::new(GfxDescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            pool_size,
        ));

        GfxDescriptorPool::new(pool_ci, "renderer")
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
        &self.set_0_bindless[*frame_label]
    }

    #[inline]
    pub fn current_perframe_descriptor_set(
        &self,
        frame_label: FrameLabel,
    ) -> &GfxDescriptorSet<PerFrameDescriptorBinding> {
        &self.set_1_perframe[*frame_label]
    }
}
