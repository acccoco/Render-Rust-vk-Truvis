use std::{marker::PhantomData, rc::Rc};

use ash::vk;

use crate::{
    foundation::{device::DeviceFunctions, mem_allocator::MemAllocator},
    render_context::RenderContext,
    resources_new::{managed_buffer::ManagedBuffer, resource_handles::BufferHandle, resource_manager::ResourceManager},
};

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
        size_of::<u16>()
    }
}
impl IndexElement for u32 {
    const VK_INDEX_TYPE: vk::IndexType = vk::IndexType::UINT32;
    fn byte_size() -> usize {
        size_of::<u32>()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct IndexBuffer<T: IndexElement> {
    buffer: BufferHandle,
    cnt: usize,
    _phantom_data: PhantomData<T>,
}
impl<T: IndexElement> IndexBuffer<T> {
    pub fn new(
        device_functions: Rc<DeviceFunctions>,
        allocator: Rc<MemAllocator>,
        resoure_mgr: &mut ResourceManager,
        index_cnt: usize,
        name: impl AsRef<str>,
    ) -> Self {
        let buffer = Self::new_managed(device_functions, allocator, index_cnt, name);
        Self {
            buffer: resoure_mgr.register_buffer(buffer),
            cnt: index_cnt,
            _phantom_data: PhantomData,
        }
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(resoure_mgr: &mut ResourceManager, data: &[T], debug_name: impl AsRef<str>) -> Self {
        let buffer =
            Self::new_managed(render_context.device_functions(), render_context.allocator(), data.len(), debug_name);
        buffer.transfer_data_sync(render_context, data);
        Self {
            buffer: resoure_mgr.register_buffer(buffer),
            cnt: data.len(),
            _phantom_data: PhantomData,
        }
    }

    fn new_managed(
        device_functions: Rc<DeviceFunctions>,
        allocator: Rc<MemAllocator>,
        index_cnt: usize,
        name: impl AsRef<str>,
    ) -> ManagedBuffer {
        let size = index_cnt * T::byte_size();
        let buffer = ManagedBuffer::new(
            device_functions.clone(),
            allocator,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            false,
            name.as_ref(),
        );
        device_functions.set_object_debug_name(buffer.handle(), format!("IndexBuffer::{}", name.as_ref()));
        buffer
    }
}
// getter
impl<T: IndexElement> IndexBuffer<T> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        T::VK_INDEX_TYPE
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.cnt
    }
}
