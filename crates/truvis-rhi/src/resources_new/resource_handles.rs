use ash::vk;
use ash::vk::Handle;

/// 这个就是 vk::Image 里面的 u64
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageHandle(pub(crate) u64);

/// 这个就是 vk::Buffer 里面的 u64
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub(crate) u64);
impl BufferHandle {
    #[inline]
    pub fn vk_buffer(&self) -> vk::Buffer {
        vk::Buffer::from_raw(self.0)
    }
}

/// 这个就是 vk::ImageView 里面的 u64
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageViewHandle(pub(crate) u64);
