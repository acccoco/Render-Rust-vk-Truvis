use crate::core::allocator::RhiAllocator;
use crate::core::command_buffer::RhiCommandBuffer;
use crate::core::debug_utils::RhiDebugType;
use crate::core::resources::buffer::RhiBuffer;
use crate::core::resources::image::RhiImageCreateInfo;
use crate::core::synchronize::RhiImageBarrier;
use ash::vk;
use std::mem::size_of_val;
use std::rc::Rc;
use vk_mem::Alloc;

/// Vulkan 格式相关的工具类
pub struct VulkanFormatUtils;
impl VulkanFormatUtils {
    /// 计算指定 Vulkan 格式下每个像素需要的字节数
    ///
    /// # Params
    /// * `format` - Vulkan 图像格式
    ///
    /// # return
    /// 每个像素的字节数
    ///
    /// # Panic
    /// 当遇到不支持的格式时会 panic
    pub fn pixel_size_in_bytes(format: vk::Format) -> usize {
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
            _ => panic!("unsupported format: {:?}", format),
        }
    }
}

pub struct ManagedImage2D {
    handle: vk::Image,
    allocation: vk_mem::Allocation,
    width: u32,
    height: u32,
    format: vk::Format,
    name: String,
}

impl RhiDebugType for ManagedImage2D {
    fn debug_type_name() -> &'static str {
        "ManagedImage2D"
    }
    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}

// 构造方法
impl ManagedImage2D {
    pub(crate) fn new(
        allocator: &RhiAllocator,
        image_info: &RhiImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        name: &str,
    ) -> Self {
        let (image, alloction) =
            unsafe { allocator.create_image(&image_info.as_info(), alloc_info).expect("Failed to create image") };
        let image = Self {
            handle: image,
            allocation: alloction,
            width: image_info.extent().width,
            height: image_info.extent().height,
            format: image_info.format(),
            name: name.to_string(),
        };
        allocator.device().debug_utils().set_debug_name(&image, name);
        image
    }
}
// Getter
impl ManagedImage2D {
    #[inline]
    pub fn handle(&self) -> vk::Image {
        self.handle
    }
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[inline]
    pub fn format(&self) -> vk::Format {
        self.format
    }
}
// 操作方法
impl ManagedImage2D {
    /// ## 实现步骤
    /// 1. 创建一个 staging buffer，用于存放待复制的数据
    /// 2. 将数据复制到 staging buffer
    /// 3. 进行图像布局转换
    /// 4. 将 staging buffer 的数据复制到图像
    /// 5. 进行图像布局转换
    pub fn copy_from_data(&self, rhi: &crate::rhi::Rhi, cmd: &RhiCommandBuffer, data: &[u8]) -> RhiBuffer {
        let pixels_cnt = self.width * self.height;
        assert_eq!(data.len(), VulkanFormatUtils::pixel_size_in_bytes(self.format) * pixels_cnt as usize);

        let mut stage_buffer = RhiBuffer::new(
            rhi,
            // TODO 使用新的 Buffer 创建方式来优化这个代码
            std::rc::Rc::new(crate::core::resources::buffer_creator::RhiBufferCreateInfo::new(
                size_of_val(data) as vk::DeviceSize,
                vk::BufferUsageFlags::TRANSFER_SRC,
            )),
            std::rc::Rc::new(vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::Auto,
                flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                ..Default::default()
            }),
            None,
            format!("{}-stage-buffer", self.name),
        );
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
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));

            let buffer_image_copy = vk::BufferImageCopy2::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: self.width,
                    height: self.height,
                    depth: 1,
                })
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            cmd.cmd_copy_buffer_to_image(
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
            cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
        }

        stage_buffer
    }
}
