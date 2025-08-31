use crate::resources::managed_buffer::RhiManagedBuffer;
use crate::resources::resource_handles::RhiBufferHandle;
use crate::resources::resource_manager::RhiResourceManager;
use crate::rhi::Rhi;
use ash::vk;
use std::marker::PhantomData;

mod private {
    /// 不允许其他类型实现该 trait
    pub trait Sealed {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
}

pub trait IndexElement: private::Sealed + Copy + 'static + Sized {
    const VK_INDEX_TYPE: vk::IndexType;
    fn byte_size() -> usize;
}
impl IndexElement for u16 {
    const VK_INDEX_TYPE: vk::IndexType = vk::IndexType::UINT16;
    fn byte_size() -> usize {
        std::mem::size_of::<u16>()
    }
}
impl IndexElement for u32 {
    const VK_INDEX_TYPE: vk::IndexType = vk::IndexType::UINT32;
    fn byte_size() -> usize {
        std::mem::size_of::<u32>()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RhiIndexBuffer<T: IndexElement> {
    buffer: RhiBufferHandle,
    cnt: usize,
    _phantom_data: PhantomData<T>,
}
impl<T: IndexElement> RhiIndexBuffer<T> {
    pub fn new(rhi: &Rhi, resoure_mgr: &mut RhiResourceManager, index_cnt: usize, name: impl AsRef<str>) -> Self {
        let buffer = Self::new_managed(rhi, index_cnt, name);
        Self {
            buffer: resoure_mgr.register_buffer(buffer),
            cnt: index_cnt,
            _phantom_data: PhantomData,
        }
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(
        rhi: &Rhi,
        resoure_mgr: &mut RhiResourceManager,
        data: &[T],
        debug_name: impl AsRef<str>,
    ) -> Self {
        let mut buffer = Self::new_managed(rhi, data.len(), debug_name);
        buffer.transfer_data_sync(rhi, data);
        Self {
            buffer: resoure_mgr.register_buffer(buffer),
            cnt: data.len(),
            _phantom_data: PhantomData,
        }
    }

    fn new_managed(rhi: &Rhi, index_cnt: usize, name: impl AsRef<str>) -> RhiManagedBuffer {
        let size = index_cnt * T::byte_size();
        let buffer = RhiManagedBuffer::new(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            false,
            name,
        );
        rhi.device.debug_utils().set_object_debug_name(buffer.handle(), format!("IndexBuffer::{}", name.as_ref()));
        buffer
    }
}
// getter
impl<T: IndexElement> RhiIndexBuffer<T> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        T::VK_INDEX_TYPE
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.cnt
    }
}
