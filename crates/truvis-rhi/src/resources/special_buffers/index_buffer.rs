use std::ops::{Deref, DerefMut};

use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType, impl_derive_buffer, render_context::RenderContext,
    resources::buffer::Buffer,
};

/// 顶点类型是 u32
pub struct IndexBuffer {
    inner: Buffer,

    /// 索引数量
    index_cnt: usize,
}

impl_derive_buffer!(IndexBuffer, Buffer, inner);
impl IndexBuffer {
    pub fn new(index_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = index_cnt * size_of::<u32>();
        let buffer = Buffer::new_device_buffer(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name,
        );

        let buffer = Self {
            inner: buffer,
            index_cnt,
        };
        let device_functions = RenderContext::get().device_functions();
        device_functions.set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(render_context: &RenderContext, data: &[u32], debug_name: impl AsRef<str>) -> Self {
        let mut index_buffer = Self::new(data.len(), debug_name);
        index_buffer.transfer_data_sync(render_context, data);
        index_buffer
    }

    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.index_cnt
    }
}

impl DebugType for IndexBuffer {
    fn debug_type_name() -> &'static str {
        "IndexBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.handle()
    }
}
