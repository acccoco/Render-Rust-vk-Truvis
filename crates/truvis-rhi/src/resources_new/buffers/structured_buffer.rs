use crate::resources_new::managed_buffer::Buffer2;
use crate::resources_new::resource_handles::BufferHandle;
use crate::resources_new::resource_manager::ResourceManager;
use ash::vk;

pub struct StructuredBufferHandle<T: bytemuck::Pod> {
    buffer: BufferHandle,

    /// 元素数量
    num: usize,

    _phantom: std::marker::PhantomData<T>,
}

// init & destroy
impl<T: bytemuck::Pod> StructuredBufferHandle<T> {
    pub fn new(resource_mgr: &mut ResourceManager, num: usize, debug_name: impl AsRef<str>) -> Self {
        let buffer = Self::new_managed(num, debug_name.as_ref());
        Self {
            buffer: resource_mgr.register_buffer(buffer),
            num,
            _phantom: std::marker::PhantomData,
        }
    }

    fn new_managed(num: usize, debug_name: impl AsRef<str>) -> Buffer2 {
        let buffer_size = (num * size_of::<T>()) as vk::DeviceSize;
        let buffer = Buffer2::new(
            buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            None,
            false,
            debug_name,
        );
        buffer
    }
}

// getters
impl<T: bytemuck::Pod> StructuredBufferHandle<T> {
    #[inline]
    pub fn num(&self) -> usize {
        self.num
    }
}

// tools
impl<T: bytemuck::Pod> StructuredBufferHandle<T> {
    pub fn mapped_slice<'a>(&self, buffer: &'a mut Buffer2) -> &'a mut [T] {
        let mapped_ptr = buffer.mapped_ptr();
        unsafe { std::slice::from_raw_parts_mut(mapped_ptr as *mut T, self.num) }
    }
}
