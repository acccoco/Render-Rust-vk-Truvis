use ash::vk;
use rhi_core::RhiCore;

use crate::{
    rhi::{queue::RhiQueueType, rhi_context::RenderCtx, rhi_struct::RhiCommandPool, swapchain::RHISwapchain},
    rhi_init_info::RhiInitInfo,
    window_system::WindowSystem,
};

mod create_utils;
mod physical_device;
mod queue;
mod rhi_context;
mod rhi_core;
pub(crate) mod rhi_struct;
mod swapchain;

static mut G_RHI: Option<Rhi> = None;


pub struct Rhi
{
    core: Option<RhiCore>,
    swapchain: Option<RHISwapchain>,

    descriptor_pool: Option<vk::DescriptorPool>,
    graphics_command_pool: Option<RhiCommandPool>,

    context: Option<RenderCtx>,
}

impl Rhi
{
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    #[inline]
    pub(crate) fn core(&self) -> &RhiCore { unsafe { self.core.as_ref().unwrap_unchecked() } }

    #[inline]
    pub fn device(&self) -> &ash::Device { self.core().device() }

    #[inline]
    pub fn vma(&self) -> &vk_mem::Allocator { self.core().vma() }

    #[inline]
    pub(crate) fn graphics_command_pool(&self) -> &RhiCommandPool { self.graphics_command_pool.as_ref().unwrap() }


    pub fn init(init_info: &RhiInitInfo)
    {
        let core = RhiCore::init(init_info);
        let mut rhi = Self {
            swapchain: Some(RHISwapchain::new(&core, init_info)),
            core: Some(core),

            descriptor_pool: None,
            graphics_command_pool: None,
            context: None,
        };

        rhi.init_descriptor_pool();
        rhi.init_command_pool();

        unsafe {
            G_RHI = Some(rhi);
        }
    }

    #[inline]
    pub fn instance() -> &'static Self { unsafe { G_RHI.as_ref().unwrap_unchecked() } }

    fn init_descriptor_pool(&mut self)
    {
        let pool_size = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: Self::MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: Self::MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: Self::MAX_MATERIAL_CNT + 32,
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
                descriptor_count: 32,
            },
        ];

        let pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_size)
            .max_sets(Self::MAX_MATERIAL_CNT + Self::MAX_VERTEX_BLENDING_MESH_CNT + 32);

        unsafe {
            self.descriptor_pool = Some(self.core().device().create_descriptor_pool(&pool_create_info, None).unwrap());
        }
    }

    fn init_command_pool(&mut self)
    {
        let command_pool = self.core().create_command_pool(
            RhiQueueType::Graphics,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            Some("rhi-core-graphics-command-pool"),
        );

        self.graphics_command_pool = Some(command_pool)
    }
}
