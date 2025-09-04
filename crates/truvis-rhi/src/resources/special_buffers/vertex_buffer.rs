use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType, impl_derive_buffer, render_context::RenderContext,
    resources::buffer::Buffer,
};

pub struct VertexBuffer<V: Sized> {
    inner: Buffer,

    /// 顶点数量
    vertex_cnt: usize,

    _phantom: PhantomData<V>,
}

impl_derive_buffer!(VertexBuffer<V: Sized>, Buffer, inner);
impl<V: Sized> VertexBuffer<V> {
    pub fn new(vertex_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = vertex_cnt * size_of::<V>();
        let buffer = Buffer::new_device_buffer(
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
        let device_functions = RenderContext::get().device_functions();
        device_functions.set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    #[inline]
    pub fn vertex_cnt(&self) -> usize {
        self.vertex_cnt
    }
}

impl<V: Sized> DebugType for VertexBuffer<V> {
    fn debug_type_name() -> &'static str {
        "VertexBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.inner.handle
    }
}
