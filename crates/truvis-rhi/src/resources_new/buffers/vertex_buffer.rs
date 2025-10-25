use crate::resources::special_buffers::vertex_buffer::VertexLayout;
use crate::resources_new::managed_buffer::Buffer2;
use crate::resources_new::resource_handles::BufferHandle;
use crate::resources_new::resource_manager::ResourceManager;
use ash::vk;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct VertexBufferHandle<L: VertexLayout> {
    buffer: BufferHandle,
    vertex_cnt: usize,
    _phantom_data: PhantomData<L>,
}

// init & destroy
impl<L: VertexLayout> VertexBufferHandle<L> {
    pub fn new(resource_mgr: &mut ResourceManager, vertex_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let buffer = Self::new_managed(vertex_cnt, debug_name.as_ref());
        Self {
            buffer: resource_mgr.register_buffer(buffer),
            vertex_cnt,
            _phantom_data: PhantomData,
        }
    }

    fn new_managed(vertex_cnt: usize, debug_name: impl AsRef<str>) -> Buffer2 {
        let buffer_size = L::buffer_size(vertex_cnt);
        Buffer2::new(
            buffer_size as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            None,
            false,
            debug_name,
        )
    }
}

impl<L: VertexLayout> VertexBufferHandle<L> {
    #[inline]
    pub fn vertex_cnt(&self) -> usize {
        self.vertex_cnt
    }
}
