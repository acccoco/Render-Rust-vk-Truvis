use crate::resources::resource_handles::ImageHandle;
use ash::vk;

pub struct ManagedImage2DView {
    handle: vk::ImageView,
    debug_name: String,
    image_handle: ImageHandle,
}
impl ManagedImage2DView {
    #[inline]
    pub fn image_handle(&self) -> ImageHandle {
        self.image_handle
    }
}
