use std::rc::Rc;

use ash::vk;
use vk_mem::Alloc;

use crate::core::allocator::RhiAllocator;
use crate::core::device::RhiDevice;
use crate::{
    core::{buffer::RhiBuffer, command_buffer::RhiCommandBuffer, synchronize::RhiImageBarrier},
    rhi::Rhi,
};

pub struct RhiImageCreateInfo {
    inner: vk::ImageCreateInfo<'static>,

    queue_family_indices: Vec<u32>,
}

impl RhiImageCreateInfo {
    #[inline]
    pub fn new_image_2d_info(extent: vk::Extent2D, format: vk::Format, usage: vk::ImageUsageFlags) -> Self {
        Self {
            inner: vk::ImageCreateInfo {
                image_type: vk::ImageType::TYPE_2D,
                format,
                extent: extent.into(),
                mip_levels: 1,
                array_layers: 1,
                samples: vk::SampleCountFlags::TYPE_1,
                tiling: vk::ImageTiling::OPTIMAL,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                ..Default::default()
            },
            queue_family_indices: Vec::new(),
        }
    }

    #[inline]
    pub fn creat_info(&self) -> &vk::ImageCreateInfo {
        &self.inner
    }

    /// getter
    #[inline]
    pub fn extent(&self) -> &vk::Extent3D {
        &self.inner.extent
    }

    /// getter
    #[inline]
    pub fn format(&self) -> vk::Format {
        self.inner.format
    }

    /// builder
    #[inline]
    pub fn queue_family_indices(mut self, queue_family_indices: &[u32]) -> Self {
        self.inner.sharing_mode = vk::SharingMode::CONCURRENT;
        self.queue_family_indices = queue_family_indices.into();

        self.inner.queue_family_index_count = self.queue_family_indices.len() as u32;
        self.inner.p_queue_family_indices = self.queue_family_indices.as_ptr();
        self
    }
}

pub struct RhiImageViewCreateInfo {
    inner: vk::ImageViewCreateInfo<'static>,
}

impl RhiImageViewCreateInfo {
    #[inline]
    pub fn new_image_view_2d_info(format: vk::Format, aspect: vk::ImageAspectFlags) -> Self {
        Self {
            inner: vk::ImageViewCreateInfo {
                format,
                view_type: vk::ImageViewType::TYPE_2D,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: aspect,
                    level_count: 1,
                    layer_count: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[inline]
    pub fn inner(&self) -> &vk::ImageViewCreateInfo {
        &self.inner
    }
}

pub struct RhiImage2D {
    pub handle: vk::Image,

    allocation: vk_mem::Allocation,

    _name: String,
    image_info: Rc<RhiImageCreateInfo>,

    allocator: Rc<RhiAllocator>,
}

impl RhiImage2D {
    pub fn new(
        rhi: &Rhi,
        image_info: Rc<RhiImageCreateInfo>,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> Self {
        let (image, alloc) = unsafe { rhi.allocator.create_image(image_info.creat_info(), alloc_info).unwrap() };

        rhi.device.debug_utils.set_object_debug_name(image, debug_name);

        Self {
            _name: debug_name.to_string(),

            handle: image,
            allocation: alloc,

            image_info,
            allocator: rhi.allocator.clone(),
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.image_info.extent().width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.image_info.extent().height
    }

    /// 根据 RGBA8_UNORM 的 data 创建 image
    pub fn from_rgba8(rhi: &Rhi, width: u32, height: u32, data: &[u8], name: &str) -> Self {
        let image = Self::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                vk::Extent2D { width, height },
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            name,
        );

        let stage_buffer = RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.graphics_command_pool.clone(),
            &rhi.graphics_queue,
            |cmd| image.transfer_data(rhi, cmd, data),
            name,
        );

        image
    }

    pub fn transfer_data(&self, rhi: &Rhi, command_buffer: &RhiCommandBuffer, data: &[u8]) -> RhiBuffer {
        let pixels_cnt = self.width() * self.height();
        assert_eq!(data.len(), Self::format_byte_count(self.image_info.format()) * pixels_cnt as usize);

        let mut stage_buffer =
            RhiBuffer::new_stage_buffer(rhi, size_of_val(data) as vk::DeviceSize, "image-stage-buffer");
        stage_buffer.transfer_data_by_mem_map(data);

        // 1. transition the image layout
        // 2. copy the buffer into the image
        // 3. transition the layout 为了让 fragment shader 可读
        {
            let image_barrier = RhiImageBarrier::new()
                .image(self.handle)
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
                    width: self.width(),
                    height: self.height(),
                    depth: 1,
                })
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            command_buffer.cmd_copy_buffer_to_image(
                &vk::CopyBufferToImageInfo2::default()
                    .src_buffer(stage_buffer.handle())
                    .dst_image(self.handle)
                    .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .regions(std::slice::from_ref(&buffer_image_copy)),
            );

            let image_barrier = RhiImageBarrier::new()
                .image(self.handle)
                .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                .dst_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ)
                .layout_transfer(vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_aspect_flag(vk::ImageAspectFlags::COLOR);
            command_buffer.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
        }

        stage_buffer
    }

    /// 计算某种 format 的一个像素需要的存储空间
    fn format_byte_count(format: vk::Format) -> usize {
        // 根据 vulkan specification 得到的 format 顺序
        const BYTE_3_FORMAT: [(vk::Format, vk::Format); 1] = [(vk::Format::R8G8B8_UNORM, vk::Format::B8G8R8_SRGB)];
        const BYTE_4_FORMAT: [(vk::Format, vk::Format); 1] = [(vk::Format::R8G8B8A8_UNORM, vk::Format::B8G8R8A8_SRGB)];
        const BYTE_6_FORMAT: [(vk::Format, vk::Format); 1] =
            [(vk::Format::R16G16B16_UNORM, vk::Format::R16G16B16_SFLOAT)];
        const BYTE_8_FORMAT: [(vk::Format, vk::Format); 1] =
            [(vk::Format::R16G16B16A16_UNORM, vk::Format::R16G16B16A16_SFLOAT)];

        let is_in_format_region = |format: vk::Format, regions: &[(vk::Format, vk::Format)]| {
            let n = format.as_raw();
            regions.iter().any(|(begin, end)| begin.as_raw() <= n && n < end.as_raw())
        };

        match format {
            f if is_in_format_region(f, &BYTE_3_FORMAT) => 3,
            f if is_in_format_region(f, &BYTE_4_FORMAT) => 4,
            f if is_in_format_region(f, &BYTE_6_FORMAT) => 6,
            f if is_in_format_region(f, &BYTE_8_FORMAT) => 8,
            _ => panic!("unsupported format."),
        }
    }
}

impl Drop for RhiImage2D {
    fn drop(&mut self) {
        unsafe { self.allocator.destroy_image(self.handle, &mut self.allocation) }
    }
}

pub struct RhiImage2DView {
    handle: vk::ImageView,

    _image: Rc<RhiImage2D>,
    _info: Rc<RhiImageViewCreateInfo>,
    _name: String,

    device: Rc<RhiDevice>,
}

impl RhiImage2DView {
    pub fn new(rhi: &Rhi, image: Rc<RhiImage2D>, mut info: RhiImageViewCreateInfo, name: String) -> Self {
        info.inner.image = image.handle;
        let handle = unsafe { rhi.device.create_image_view(&info.inner, None).unwrap() };
        rhi.device.debug_utils.set_object_debug_name(handle, &name);
        Self {
            handle,
            _image: image,
            _info: Rc::new(info),
            _name: name,
            device: rhi.device.clone(),
        }
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::ImageView {
        self.handle
    }
}

impl Drop for RhiImage2DView {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.handle, None);
        }
    }
}
