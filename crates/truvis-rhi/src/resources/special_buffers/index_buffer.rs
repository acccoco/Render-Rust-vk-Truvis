use std::ops::{Deref, DerefMut};

use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType, impl_derive_buffer, render_context::RenderContext,
    resources::buffer::Buffer,
};

pub trait IndexType: Sized + Copy {
    const VK_INDEX_TYPE: vk::IndexType;
    fn byte_size() -> usize;
}
impl IndexType for u16 {
    const VK_INDEX_TYPE: vk::IndexType = vk::IndexType::UINT16;
    fn byte_size() -> usize {
        size_of::<u16>()
    }
}
impl IndexType for u32 {
    const VK_INDEX_TYPE: vk::IndexType = vk::IndexType::UINT32;
    fn byte_size() -> usize {
        size_of::<u32>()
    }
}

/// 顶点类型是 u32
pub struct IndexBuffer<T: IndexType> {
    inner: Buffer,

    /// 索引数量
    index_cnt: usize,

    _phantom: std::marker::PhantomData<T>,
}

impl_derive_buffer!(IndexBuffer<T: IndexType>, Buffer, inner);

// init & destroy
impl<T: IndexType> IndexBuffer<T> {
    pub fn new(index_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = index_cnt * size_of::<u32>();
        let buffer = Buffer::new(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            None,
            false,
            debug_name,
        );

        let buffer = Self {
            inner: buffer,
            index_cnt,
            _phantom: std::marker::PhantomData,
        };
        let device_functions = RenderContext::get().device_functions();
        device_functions.set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(data: &[u32], debug_name: impl AsRef<str>) -> Self {
        let mut index_buffer = Self::new(data.len(), debug_name);
        index_buffer.transfer_data_sync(data);
        index_buffer
    }
}
// getter
impl<T: IndexType> IndexBuffer<T> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        T::VK_INDEX_TYPE
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.index_cnt
    }
}

impl<T: IndexType> DebugType for IndexBuffer<T> {
    fn debug_type_name() -> &'static str {
        "IndexBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.vk_buffer()
    }
}

pub type Index32Buffer = IndexBuffer<u32>;
pub type Index16Buffer = IndexBuffer<u16>;
