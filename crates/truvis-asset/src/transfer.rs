use crate::handle::TextureHandle;
use crate::loader::RawAssetData;
use ash::vk;
use std::collections::VecDeque;
use truvis_gfx::commands::barrier::GfxImageBarrier;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::commands::command_pool::GfxCommandPool;
use truvis_gfx::commands::semaphore::GfxSemaphore;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::handles::{BufferHandle, ImageHandle};
use truvis_gfx::resources::resource_data::BufferType;

struct PendingUpload {
    target_value: u64,
    _staging_buffer: BufferHandle,
    command_buffer: GfxCommandBuffer,
    handle: TextureHandle,
    image: ImageHandle,
}

/// 传输管理器
///
/// 负责管理 Vulkan Transfer Queue 的异步上传任务。
/// 核心机制:
/// 1. 使用 Timeline Semaphore 跟踪上传进度，避免为每个任务创建 Fence。
/// 2. 维护一个 Pending 队列，在 update() 中检查 Semaphore 值来回收资源。
/// 3. 自动处理 Staging Buffer 的创建和销毁。
/// 4. 处理 Image Layout 转换 (Undefined -> TransferDst -> ShaderReadOnly)。
pub struct AssetTransferManager {
    command_pool: GfxCommandPool,
    timeline_semaphore: GfxSemaphore,
    next_timeline_value: u64,

    pending_uploads: VecDeque<PendingUpload>,
}

impl Default for AssetTransferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetTransferManager {
    pub fn new() -> Self {
        let gfx = Gfx::get();
        let transfer_queue = gfx.transfer_queue();

        // 1. 创建 Command Pool
        let command_pool = GfxCommandPool::new(
            transfer_queue.queue_family().clone(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            "AssetTransferPool",
        );

        // 2. 创建 Timeline Semaphore
        let timeline_semaphore = GfxSemaphore::new_timeline(0, "AssetTransferTimeline");

        Self {
            command_pool,
            timeline_semaphore,
            next_timeline_value: 1,
            pending_uploads: VecDeque::new(),
        }
    }

    /// 提交纹理上传任务
    ///
    /// 流程:
    /// 1. 创建 HostVisible 的 Staging Buffer 并写入像素数据。
    /// 2. 创建 DeviceLocal 的目标 Image。
    /// 3. 分配并录制 Command Buffer:
    ///    - Barrier: Image Undefined -> TransferDst
    ///    - Copy: Buffer -> Image
    ///    - Barrier: Image TransferDst -> ShaderReadOnly
    /// 4. 提交到 Transfer Queue，并设置 Timeline Semaphore 的 Signal 操作。
    pub fn upload_texture(&mut self, data: RawAssetData) -> anyhow::Result<()> {
        let _span = tracy_client::span!("upload_texture");
        let gfx = Gfx::get();
        let mut rm = gfx.resource_manager();

        // 1. 创建目标 Image
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(data.format)
            .extent(vk::Extent3D {
                width: data.extent.width,
                height: data.extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image_handle = rm.create_image(&create_info, &alloc_info, "AssetTexture");

        // 2. 分配 Command Buffer
        let command_buffer = GfxCommandBuffer::new(&self.command_pool, "AssetUploadCmd");

        // 3. 录制命令
        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "AssetUpload");

        // 创建 Staging Buffer 并录制复制命令
        let staging_buffer = Self::record_transfer_commands(
            &mut rm,
            &command_buffer,
            image_handle,
            &data.pixels,
            data.extent.width,
            data.extent.height,
        );

        command_buffer.end();

        // 4. 提交命令
        let target_value = self.next_timeline_value;
        self.next_timeline_value += 1;

        let submit_info = GfxSubmitInfo::new(&[command_buffer.clone()]).signal(
            &self.timeline_semaphore,
            vk::PipelineStageFlags2::ALL_COMMANDS,
            Some(target_value),
        );

        gfx.transfer_queue().submit(vec![submit_info], None);

        // 5. 记录 Pending Upload
        self.pending_uploads.push_back(PendingUpload {
            target_value,
            _staging_buffer: staging_buffer,
            command_buffer,
            handle: data.handle,
            image: image_handle,
        });

        Ok(())
    }

    fn record_transfer_commands(
        rm: &mut truvis_gfx::resources::manager::ResourceManager,
        command_buffer: &GfxCommandBuffer,
        image_handle: ImageHandle,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> BufferHandle {
        // 创建 Staging Buffer
        let stage_buffer_handle = rm.create_buffer(
            data.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            "image-stage-buffer",
        );

        // 写入数据
        {
            let buffer_res = rm.get_buffer_mut(stage_buffer_handle).unwrap();
            if let Some(ptr) = buffer_res.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
                    // Flush
                    let allocator = Gfx::get().allocator();
                    allocator.flush_allocation(&buffer_res.allocation, 0, data.len() as vk::DeviceSize).unwrap();
                }
            }
        }

        let stage_vk_buffer = rm.get_buffer(stage_buffer_handle).unwrap().buffer;
        let image_vk = rm.get_image(image_handle).unwrap().image;

        // 1. transition the image layout
        // 2. copy the buffer into the image
        // 3. transition the layout 为了让 fragment shader 可读
        {
            let image_barrier = GfxImageBarrier::new()
                .image(image_vk)
                .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR);
            command_buffer.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));

            let buffer_image_copy = vk::BufferImageCopy2::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            unsafe {
                Gfx::get().gfx_device().cmd_copy_buffer_to_image2(
                    command_buffer.vk_handle(),
                    &vk::CopyBufferToImageInfo2::default()
                        .src_buffer(stage_vk_buffer)
                        .dst_image(image_vk)
                        .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .regions(std::slice::from_ref(&buffer_image_copy)),
                );
            }

            let image_barrier = GfxImageBarrier::new()
                .image(image_vk)
                .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                .dst_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ)
                .layout_transfer(vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR);
            command_buffer.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
        }

        stage_buffer_handle
    }

    /// 检查上传任务状态
    ///
    /// 必须每帧调用。
    /// 返回已完成上传的资源列表 (Handle + Image)。
    /// 同时负责回收 Staging Buffer 和 Command Buffer。
    pub fn update(&mut self) -> Vec<(TextureHandle, ImageHandle)> {
        let _span = tracy_client::span!("TransferManager::update");
        let gfx = Gfx::get();
        let device = gfx.gfx_device();

        // 查询当前 Timeline Semaphore 的值 (非阻塞)
        let current_value =
            unsafe { device.get_semaphore_counter_value(self.timeline_semaphore.handle()).unwrap_or(0) };

        let mut finished_uploads = Vec::new();

        while let Some(upload) = self.pending_uploads.front() {
            if current_value >= upload.target_value {
                // 上传完成
                let upload = self.pending_uploads.pop_front().unwrap();

                // 释放 Command Buffer
                self.command_pool.free_command_buffers(vec![upload.command_buffer]);

                // 销毁 Staging Buffer
                Gfx::get().resource_manager().destroy_buffer_immediate(upload._staging_buffer);

                finished_uploads.push((upload.handle, upload.image));
            } else {
                // 队列是有序的，如果队头未完成，后续肯定也未完成
                break;
            }
        }

        finished_uploads
    }
}

impl Drop for AssetTransferManager {
    fn drop(&mut self) {
        self.timeline_semaphore.clone().destroy();
        self.command_pool.destroy();
    }
}
