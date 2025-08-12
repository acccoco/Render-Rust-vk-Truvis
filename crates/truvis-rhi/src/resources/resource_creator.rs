use crate::core::allocator::RhiAllocator;
use crate::core::resources::image::RhiImageCreateInfo;
use crate::resources::managed_image::ManagedImage2D;
use crate::resources::resource_handles::ImageHandle;
use crate::resources::resource_manager::RhiResourceManager;

impl ResourceCreator {
    #[inline]
    pub fn new_image_2d(
        allocator: &RhiAllocator,
        manager: &mut RhiResourceManager,
        image_info: &RhiImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        name: &str,
    ) -> ImageHandle {
        manager.register_image(ManagedImage2D::new(allocator, image_info, alloc_info, name))
    }
}

pub struct ResourceCreator;