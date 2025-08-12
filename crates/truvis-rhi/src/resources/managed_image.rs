use crate::core::allocator::RhiAllocator;
use crate::core::command_buffer::RhiCommandBuffer;
use crate::core::debug_utils::RhiDebugType;
use crate::core::resources::image::RhiImageCreateInfo;
use ash::vk;
use vk_mem::Alloc;

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
    pub fn copy_from_data(&self, cmd: &RhiCommandBuffer, data: &[u8]) {
        todo!()
    }
}
