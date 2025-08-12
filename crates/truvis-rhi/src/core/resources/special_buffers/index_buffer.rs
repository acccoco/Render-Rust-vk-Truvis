use crate::core::debug_utils::RhiDebugType;
use crate::core::resources::buffer::RhiBuffer;
use crate::impl_derive_buffer;
use crate::rhi::Rhi;
use ash::vk;
use std::ops::Deref;
use std::ops::DerefMut;

/// 顶点类型是 u32
pub struct RhiIndexBuffer {
    inner: RhiBuffer,

    /// 索引数量
    index_cnt: usize,
}

impl_derive_buffer!(RhiIndexBuffer, RhiBuffer, inner);
impl RhiIndexBuffer {
    pub fn new(rhi: &Rhi, index_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = index_cnt * size_of::<u32>();
        let buffer = RhiBuffer::new_device_buffer(
            rhi,
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
        rhi.device.debug_utils().set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(rhi: &Rhi, data: &[u32], debug_name: impl AsRef<str>) -> Self {
        let mut index_buffer = Self::new(rhi, data.len(), debug_name);
        index_buffer.transfer_data_sync(rhi, data);
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
