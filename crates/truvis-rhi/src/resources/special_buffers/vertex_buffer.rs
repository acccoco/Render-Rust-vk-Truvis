use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType, impl_derive_buffer, render_context::RenderContext,
    resources::buffer::Buffer,
};

/// Vertex Buffer 中顶点布局的 trait 定义
pub trait VertexLayout {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription>;

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription>;

    /// 整个 Buffer 的大小
    fn buffer_size(vertex_cnt: usize) -> usize;

    /// position 属性的 stride
    fn pos_stride() -> u32 {
        unimplemented!()
    }

    /// position 属性在 Buffer 中的偏移量
    fn pos_offset(_vertex_cnt: usize) -> vk::DeviceSize {
        unimplemented!()
    }
    /// normal 属性在 Buffer 中的偏移量
    fn normal_offset(_vertex_cnt: usize) -> vk::DeviceSize {
        unimplemented!()
    }
    /// tangent 属性在 Buffer 中的偏移量
    fn tangent_offset(_vertex_cnt: usize) -> vk::DeviceSize {
        unimplemented!()
    }
    /// uv 属性在 Buffer 中的偏移量
    fn uv_offset(_vertex_cnt: usize) -> vk::DeviceSize {
        unimplemented!()
    }
}

pub struct VertexBuffer<L: VertexLayout> {
    inner: Buffer,
    /// 顶点数量
    vertex_cnt: usize,
    _phantom: PhantomData<L>,
}
impl_derive_buffer!(VertexBuffer<L: VertexLayout>, Buffer, inner);
impl<L: VertexLayout> VertexBuffer<L> {
    pub fn new(vertex_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let buffer_size = L::buffer_size(vertex_cnt);
        let buffer = Buffer::new_device_buffer(
            buffer_size as vk::DeviceSize,
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

    #[inline]
    pub fn pos_address(&self) -> vk::DeviceSize {
        self.device_address() + L::pos_offset(self.vertex_cnt)
    }

    #[inline]
    pub fn normal_address(&self) -> vk::DeviceSize {
        self.device_address() + L::normal_offset(self.vertex_cnt)
    }

    #[inline]
    pub fn tangent_address(&self) -> vk::DeviceSize {
        self.device_address() + L::tangent_offset(self.vertex_cnt)
    }

    #[inline]
    pub fn uv_address(&self) -> vk::DeviceSize {
        self.device_address() + L::uv_offset(self.vertex_cnt)
    }
}

impl<L: VertexLayout> DebugType for VertexBuffer<L> {
    fn debug_type_name() -> &'static str {
        "VertexBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.inner.handle
    }
}
