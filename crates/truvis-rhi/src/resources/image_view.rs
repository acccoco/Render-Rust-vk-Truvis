use std::rc::Rc;

use ash::vk;

use crate::foundation::{debug_messenger::DebugType, device::DeviceFunctions, mem_allocator::MemAllocator};

impl ImageViewCreateInfo
{
    #[inline]
    pub fn new_image_view_2d_info(format: vk::Format, aspect: vk::ImageAspectFlags) -> Self
    {
        Self {
            inner: vk::ImageViewCreateInfo {
                format,
                view_type: vk::ImageViewType::TYPE_2D,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: aspect,
                    level_count: 1,
                    layer_count: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[inline]
    pub fn inner(&self) -> &vk::ImageViewCreateInfo<'_>
    {
        &self.inner
    }
}

#[derive(PartialOrd, PartialEq, Hash, Copy, Clone, Ord, Eq, Debug)]
pub struct Image2DViewUUID(pub uuid::Uuid);

impl std::fmt::Display for Image2DViewUUID
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "image2d-view-uuid-{}", self.0)
    }
}

pub struct Image2DView
{
    handle: vk::ImageView,
    uuid: Image2DViewUUID,

    _info: Rc<ImageViewCreateInfo>,
    _name: String,

    device_functions: Rc<DeviceFunctions>,
}

impl Drop for Image2DView
{
    fn drop(&mut self)
    {
        unsafe {
            self.device_functions.destroy_image_view(self.handle, None);
        }
    }
}

impl DebugType for Image2DView
{
    fn debug_type_name() -> &'static str
    {
        "RhiImage2DView"
    }

    fn vk_handle(&self) -> impl vk::Handle
    {
        self.handle
    }
}

impl Image2DView
{
    pub fn new(
        device_functions: Rc<DeviceFunctions>,
        image: vk::Image,
        mut info: ImageViewCreateInfo,
        name: impl AsRef<str>,
    ) -> Self
    {
        info.inner.image = image;
        let handle = unsafe { device_functions.create_image_view(&info.inner, None).unwrap() };
        let image_view = Self {
            handle,
            uuid: Image2DViewUUID(uuid::Uuid::new_v4()),
            _info: Rc::new(info),
            _name: name.as_ref().to_string(),
            device_functions: device_functions.clone(),
        };
        device_functions.set_debug_name(&image_view, &name);
        image_view
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::ImageView
    {
        self.handle
    }

    #[inline]
    pub fn uuid(&self) -> Image2DViewUUID
    {
        self.uuid
    }
}

pub enum Image2DViewContainer
{
    Own(Box<Image2DView>),
    Shared(Rc<Image2DView>),
    Raw(vk::ImageView),
}

impl Image2DViewContainer
{
    #[inline]
    pub fn vk_image_view(&self) -> vk::ImageView
    {
        match self {
            Image2DViewContainer::Own(view) => view.handle(),
            Image2DViewContainer::Shared(view) => view.handle(),
            Image2DViewContainer::Raw(view) => *view,
        }
    }
}

pub struct ImageViewCreateInfo
{
    inner: vk::ImageViewCreateInfo<'static>,
}
