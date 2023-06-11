use std::ffi::CStr;

use ash::{extensions::khr::Swapchain, vk};
use raw_window_handle::HasRawDisplayHandle;
use rhi_core::RhiCore;

use crate::{
    rhi::{queue::RhiQueueType, render_ctx::RenderCtx, swapchain::RHISwapchain},
    rhi_init_info::RhiInitInfo,
    window_system::WindowSystem,
};

mod create_info_uitls;
mod physical_device;
mod queue;
mod render_ctx;
mod rhi_core;
mod swapchain;

pub struct Rhi
{
    core: Option<RhiCore>,
    swapchain: Option<RHISwapchain>,
    descriptor_pool: Option<vk::DescriptorPool>,
    graphics_command_pool: Option<vk::CommandPool>,

    context: Option<RenderCtx>,
}

impl Rhi
{
    const MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const MAX_MATERIAL_CNT: u32 = 256;

    pub fn core(&self) -> &RhiCore { unsafe { self.core.as_ref().unwrap_unchecked() } }

    pub fn init(init_info: &RhiInitInfo) -> Self
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


        rhi
    }

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
        self.graphics_command_pool = Some(self.core().create_command_pool(
            RhiQueueType::Graphics,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            Some("rhi-core-graphics-command-pool"),
        ));
    }
}
