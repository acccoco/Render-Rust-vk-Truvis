use std::marker::PhantomData;

use crate::{
    render_context::RenderContext,
    resources_new::{managed_buffer::Buffer2, resource_handles::BufferHandle, resource_manager::ResourceManager},
};
use ash::vk;

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

#[derive(Debug, Copy, Clone)]
pub struct IndexBufferHandle<T: IndexType> {
    buffer: BufferHandle,
    cnt: usize,
    _phantom_data: PhantomData<T>,
}
impl<T: IndexType> IndexBufferHandle<T> {
    pub fn new(resoure_mgr: &mut ResourceManager, index_cnt: usize, name: impl AsRef<str>) -> Self {
        let buffer = Self::new_managed(index_cnt, name);
        Self {
            buffer: resoure_mgr.register_buffer(buffer),
            cnt: index_cnt,
            _phantom_data: PhantomData,
        }
    }

    pub fn new_with_buffer(resource_mgr: &mut ResourceManager, index_cnt: usize, buffer: Buffer2) -> Self {
        Self {
            buffer: resource_mgr.register_buffer(buffer),
            cnt: index_cnt,
            _phantom_data: PhantomData,
        }
    }

    pub fn new_managed(index_cnt: usize, name: impl AsRef<str>) -> Buffer2 {
        let size = index_cnt * T::byte_size();
        let buffer = Buffer2::new(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            None,
            false,
            name.as_ref(),
        );
        RenderContext::get()
            .device_functions()
            .set_object_debug_name(buffer.vk_buffer(), format!("IndexBuffer::{}", name.as_ref()));
        buffer
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(resoure_mgr: &mut ResourceManager, data: &[T], debug_name: impl AsRef<str>) -> Self {
        let buffer = Self::new_managed(data.len(), debug_name);
        buffer.transfer_data_sync(data);
        Self {
            buffer: resoure_mgr.register_buffer(buffer),
            cnt: data.len(),
            _phantom_data: PhantomData,
        }
    }
}
// getter
impl<T: IndexType> IndexBufferHandle<T> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        T::VK_INDEX_TYPE
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.cnt
    }

    #[inline]
    pub fn device_address(&self, resoure_mgr: &ResourceManager) -> vk::DeviceAddress {
        resoure_mgr.get_buffer(self.buffer).as_ref().unwrap().device_address()
    }

    #[inline]
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer.vk_buffer()
    }

    #[inline]
    pub fn buffer_handle(&self) -> BufferHandle {
        self.buffer
    }
}

pub type Index32BufferHandle = IndexBufferHandle<u32>;
pub type Index16BufferHandle = IndexBufferHandle<u16>;
