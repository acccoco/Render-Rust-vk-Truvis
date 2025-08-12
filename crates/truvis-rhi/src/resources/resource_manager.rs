use crate::resources::managed_buffer::ManagedBuffer;
use crate::resources::managed_image::ManagedImage2D;
use crate::resources::managed_image_view::ManagedImage2DView;
use crate::resources::resource_handles::{BufferHandle, ImageHandle, ImageViewHandle};
use std::collections::HashMap;

pub struct RhiResourceManager {
    images: HashMap<ImageHandle, ManagedImage2D>,
    buffers: HashMap<BufferHandle, ManagedBuffer>,
    image_views: HashMap<ImageViewHandle, ManagedImage2DView>,

    /// 每个 ImageHandle 对应的 ImageViewHandle 列表
    image_to_views: HashMap<ImageHandle, Vec<ImageViewHandle>>,

    next_image_id: u64,
    next_buffer_id: u64,
    next_view_id: u64,
}
impl Default for RhiResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
impl RhiResourceManager {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
            buffers: HashMap::new(),
            image_views: HashMap::new(),
            image_to_views: HashMap::new(),
            next_image_id: 0,
            next_buffer_id: 0,
            next_view_id: 0,
        }
    }

    pub fn register_image(&mut self, image: ManagedImage2D) -> ImageHandle {
        let handle = ImageHandle(self.next_image_id);
        self.images.insert(handle, image);
        self.next_image_id += 1;
        handle
    }

    pub fn register_buffer(&mut self, buffer: ManagedBuffer) -> BufferHandle {
        let handle = BufferHandle(self.next_buffer_id);
        self.buffers.insert(handle, buffer);
        self.next_buffer_id += 1;
        handle
    }

    pub fn register_image_view(&mut self, view: ManagedImage2DView) -> ImageViewHandle {
        let image_handle = view.image_handle();
        let handle = ImageViewHandle(self.next_view_id);
        self.image_views.insert(handle, view);
        self.next_view_id += 1;

        // 维护 image_to_views 映射
        if let Some(views) = self.image_to_views.get_mut(&image_handle) {
            views.push(handle);
        } else {
            self.image_to_views.insert(image_handle, vec![handle]);
        }

        handle
    }

    pub fn get_image(&self, handle: ImageHandle) -> Option<&ManagedImage2D> {
        self.images.get(&handle)
    }

    pub fn get_image_mut(&mut self, handle: ImageHandle) -> Option<&mut ManagedImage2D> {
        self.images.get_mut(&handle)
    }

    pub fn get_buffer(&self, handle: BufferHandle) -> Option<&ManagedBuffer> {
        self.buffers.get(&handle)
    }

    pub fn get_buffer_mut(&mut self, handle: BufferHandle) -> Option<&mut ManagedBuffer> {
        self.buffers.get_mut(&handle)
    }

    pub fn get_image_view(&self, handle: ImageViewHandle) -> Option<&ManagedImage2DView> {
        self.image_views.get(&handle)
    }
}
