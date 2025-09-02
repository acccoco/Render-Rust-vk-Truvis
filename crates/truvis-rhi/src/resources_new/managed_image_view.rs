use ash::vk;

use crate::resources_new::resource_handles::ImageHandle;

pub struct ManagedImageView
{
    handle: vk::ImageView,
    debug_name: String,
    image_handle: ImageHandle,
}
impl ManagedImageView
{
    #[inline]
    pub fn image_handle(&self) -> ImageHandle
    {
        self.image_handle
    }
}
