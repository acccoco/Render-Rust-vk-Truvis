use ash::vk;
use vk_mem::Alloc;

use crate::framework::{core::buffer::RhiBuffer, rhi::Rhi};

pub struct RhiImage2DInfo
{
    format: vk::Format,
    extent: vk::Extent2D,
    usage: vk::ImageUsageFlags,
    tiling: vk::ImageTiling,
    samples: vk::SampleCountFlags,
}

impl From<vk::ImageCreateInfo> for RhiImage2DInfo
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

    image: vk::Image,
    alloc: vk_mem::Allocation,

    image_info: RhiImage2DInfo,
}


impl RhiImage2D
{
    pub fn new(
        image_info: &vk::ImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> Self
    {
        let rhi = Rhi::instance();
        let (image, alloc) = unsafe { rhi.vma().create_image(image_info, alloc_info).unwrap() };

        rhi.set_debug_name(image, debug_name);

        Self {
            name: debug_name.to_string(),

            image,
            alloc,

            image_info: (*image_info).into(),
        }
    }

    pub fn transfer_data(&mut self, data: &[u8])
    {
        let pixels_cnt = self.image_info.extent.width * self.image_info.extent.height;
        assert_eq!(
            data.len(),
            Self::format_byte_count(self.image_info.format) * pixels_cnt as usize
        );

        let mut stage_buffer = RhiBuffer::new_stage_buffer(
            std::mem::size_of_val(data) as vk::DeviceSize,
            "image-stage-buffer",
        );
        stage_buffer.map();

        // TODO
    }

    /// 某种格式的像素需要的字节数
    fn format_byte_count(format: vk::Format) -> usize
    {
        // 根据 vulkan specification 得到的 format 顺序
        const BYTE_3_FORMAT: [(vk::Format, vk::Format); 1] =
            [(vk::Format::R8G8B8_UNORM, vk::Format::B8G8R8_SRGB)];
        const BYTE_4_FORMAT: [(vk::Format, vk::Format); 1] =
            [(vk::Format::R8G8B8A8_UNORM, vk::Format::B8G8R8A8_SRGB)];
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
