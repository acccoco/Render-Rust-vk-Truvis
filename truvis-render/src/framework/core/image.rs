use ash::vk;
use vk_mem::Alloc;

use crate::framework::{
    core::{buffer::RhiBuffer, command_buffer::RhiCommandBuffer},
    rhi::Rhi,
};

pub struct RhiImage2DInfo
{
    format: vk::Format,
    extent: vk::Extent2D,
    usage: vk::ImageUsageFlags,
    tiling: vk::ImageTiling,
    samples: vk::SampleCountFlags,
}

impl From<vk::ImageCreateInfo<'static>> for RhiImage2DInfo
{
    fn from(value: vk::ImageCreateInfo) -> Self
    {
        Self {
            format: value.format,
            extent: vk::Extent2D {
                width: value.extent.width,
                height: value.extent.height,
            },
            usage: value.usage,
            tiling: value.tiling,
            samples: value.samples,
        }
    }
}

pub struct RhiImage2D
{
    name: String,

    pub handle: vk::Image,

    alloc: vk_mem::Allocation,

    image_info: RhiImage2DInfo,

    rhi: &'static Rhi,
}


impl RhiImage2D
{
    pub fn new(
        rhi: &'static Rhi,
        // FIXME 声明周期问题
        image_info: &vk::ImageCreateInfo<'static>,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> Self
    {
        let (image, alloc) = unsafe { rhi.vma().create_image(image_info, alloc_info).unwrap() };

        rhi.set_debug_name(image, debug_name);

        Self {
            name: debug_name.to_string(),

            handle: image,
            alloc,

            image_info: (*image_info).into(),
            rhi,
        }
    }

    #[inline]
    pub fn width(&self) -> u32
    {
        self.image_info.extent.width
    }

    #[inline]
    pub fn height(&self) -> u32
    {
        self.image_info.extent.height
    }

    /// 根据 RGBA8_UNORM 的 data 创建 image
    pub fn from_rgba8(rhi: &'static Rhi, width: u32, height: u32, data: &[u8], name: &str) -> Self
    {
        let mut image = Self::new(
            rhi,
            &vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            name,
        );

        let stage_buffer =
            RhiCommandBuffer::one_time_exec(rhi, vk::QueueFlags::GRAPHICS, |cmd| image.transfer_data(cmd, data), name);
        stage_buffer.destroy();

        image
    }

    pub fn transfer_data(&mut self, command_buffer: &mut RhiCommandBuffer, data: &[u8]) -> RhiBuffer
    {
        let pixels_cnt = self.image_info.extent.width * self.image_info.extent.height;
        assert_eq!(data.len(), Self::format_byte_count(self.image_info.format) * pixels_cnt as usize);

        let mut stage_buffer =
            RhiBuffer::new_stage_buffer(self.rhi, std::mem::size_of_val(data) as vk::DeviceSize, "image-stage-buffer");
        stage_buffer.transfer_data_by_mem_map(data);

        // 1. transition the image layout
        // 2. copy the buffer into the image
        // 3. transition the layout 为了让 fragment shader 可读
        {
            let image_barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                .src_access_mask(vk::AccessFlags2::empty())
                .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(self.handle)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
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
                    .src_buffer(stage_buffer.handle)
                    .dst_image(self.handle)
                    .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .regions(std::slice::from_ref(&buffer_image_copy)),
            );

            let image_barrier = vk::ImageMemoryBarrier2 {
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags2::SHADER_READ,
                src_stage_mask: vk::PipelineStageFlags2::TRANSFER,
                dst_stage_mask: vk::PipelineStageFlags2::FRAGMENT_SHADER,
                ..image_barrier
            };
            command_buffer.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
        }

        stage_buffer
    }

    /// 某种格式的像素需要的字节数
    fn format_byte_count(format: vk::Format) -> usize
    {
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
