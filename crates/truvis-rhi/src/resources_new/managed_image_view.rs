use ash::vk;

use crate::resources_new::resource_handles::ImageHandle;

pub struct ImageView2 {
    handle: vk::ImageView,
    debug_name: String,
    image_handle: ImageHandle,
}
impl ImageView2 {
    #[inline]
    pub fn vk_image_(&self) -> ImageHandle {
        self.image_handle
    }

    #[inline]
    pub fn vk_image_view(&self) -> vk::ImageView {
        self.handle
    }
}
