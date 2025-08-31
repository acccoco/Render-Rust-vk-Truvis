use crate::resources::resource_handles::RhiImageHandle;
use ash::vk;

pub struct RhiManagedImageView {
    handle: vk::ImageView,
    debug_name: String,
    image_handle: RhiImageHandle,
}
impl RhiManagedImageView {
    #[inline]
    pub fn image_handle(&self) -> RhiImageHandle {
        self.image_handle
    }
}
