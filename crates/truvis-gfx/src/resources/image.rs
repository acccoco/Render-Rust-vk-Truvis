use ash::vk;

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
