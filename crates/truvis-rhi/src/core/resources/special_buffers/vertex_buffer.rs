use crate::core::debug_utils::RhiDebugType;
use crate::core::resources::buffer::RhiBuffer;
use crate::impl_derive_buffer;
use crate::rhi::Rhi;
use ash::vk;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct RhiVertexBuffer<V: Sized> {
    inner: RhiBuffer,

    /// 顶点数量
    vertex_cnt: usize,

    _phantom: PhantomData<V>,
}

impl_derive_buffer!(RhiVertexBuffer<V: Sized>, RhiBuffer, inner);
impl<V: Sized> RhiVertexBuffer<V> {
    pub fn new(rhi: &Rhi, vertex_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = vertex_cnt * size_of::<V>();
        let buffer = RhiBuffer::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name,
        );

        let buffer = Self {
            inner: buffer,
            vertex_cnt,
            _phantom: PhantomData,
        };
        rhi.device.debug_utils().set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    #[inline]
    pub fn vertex_cnt(&self) -> usize {
        self.vertex_cnt
    }
}
