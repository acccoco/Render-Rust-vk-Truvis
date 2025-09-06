use ash::vk;
use vk_mem::Alloc;

use crate::{
    foundation::{device::DeviceFunctions, mem_allocator::MemAllocator},
    render_context::RenderContext,
};

pub struct ManagedBuffer {
    vk_handle: vk::Buffer,
    allocation: vk_mem::Allocation,

    size: vk::DeviceSize,

    map_ptr: Option<*mut u8>,
    device_addr: Option<vk::DeviceAddress>,

    debug_name: String,

    #[cfg(debug_assertions)]
    destroyed: bool,
}
impl ManagedBuffer {
    /// # Note
    /// - 默认对齐到 8 字节
    /// - 优先使用 device memory
    pub fn new(
        buffer_size: vk::DeviceSize,
        buffer_usage: vk::BufferUsageFlags,
        mem_map: bool,
        name: impl AsRef<str>,
    ) -> Self {
        let buffer_ci = vk::BufferCreateInfo::default().size(buffer_size).usage(buffer_usage);
        let alloc_ci = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            flags: if mem_map {
                vk_mem::AllocationCreateFlags::empty()
            } else {
                vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
            },
            ..Default::default()
        };

        let (buffer, alloc) =
            unsafe { RenderContext::get().allocator.create_buffer_with_alignment(&buffer_ci, &alloc_ci, 8).unwrap() };
        RenderContext::get().device_functions().set_object_debug_name(buffer, format!("Buffer::{}", name.as_ref()));
        Self {
            vk_handle: buffer,
            allocation: alloc,
            size: buffer_size,
            map_ptr: None,
            device_addr: None,
            debug_name: name.as_ref().to_string(),

            #[cfg(debug_assertions)]
            destroyed: false,
        }
    }

    #[inline]
    pub fn new_stage_buffer(buffer_size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        Self::new(buffer_size, vk::BufferUsageFlags::TRANSFER_SRC, true, name)
    }

    pub fn destroy(mut self) {
        unsafe {
            RenderContext::get().allocator().destroy_buffer(self.vk_handle, &mut self.allocation);
        }
    }
}
/// getter
impl ManagedBuffer {
    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self.vk_handle
    }
    #[inline]
    pub fn size(&self) -> vk::DeviceSize {
        self.size
    }
    #[inline]
    pub fn mapped_ptr(&self) -> *mut u8 {
        self.map_ptr.unwrap_or_else(|| {
            panic!("Buffer is not mapped, please call map() before using mapped_ptr()");
        })
    }
    #[inline]
    pub fn device_address(&self, device: &DeviceFunctions) -> vk::DeviceAddress {
        self.device_addr.unwrap_or_else(|| unsafe {
            device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(self.vk_handle))
        })
    }
}
/// tools
impl ManagedBuffer {
    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到
    /// self
    ///
    /// sync 表示这个函数是同步等待的，会阻塞运行
    ///
    /// # Note
    /// * 避免使用这个将 *小块* 数据从内存传到 GPU，推荐使用 cmd transfer
    /// * 这个应该是用来传输大块数据的
    pub fn transfer_data_sync(&self, data: &[impl Sized + Copy]) {
        let mut stage_buffer =
            Self::new_stage_buffer(size_of_val(data) as vk::DeviceSize, format!("{}-stage-buffer", self.debug_name));
        stage_buffer.transfer_data_by_mem_map(data, &RenderContext::get().allocator());

        RenderContext::get().one_time_exec(
            |cmd| {
                cmd.cmd_copy_buffer(
                    &stage_buffer,
                    self,
                    &[vk::BufferCopy {
                        size: size_of_val(data) as vk::DeviceSize,
                        ..Default::default()
                    }],
                );
            },
            format!("{}-transfer-data", &self.debug_name),
        );

        stage_buffer.destroy();
    }

    /// 确保 `[T]` 的内存布局在 CPU 和 GPU 是一致的
    ///
    /// 如果需要处理内存对齐的问题，考虑使用 `ash::util::Align`
    pub fn transfer_data_by_mem_map<T>(&mut self, data: &[T], allocator: &MemAllocator)
    where
        T: Sized + Copy,
    {
        // 准备好源数据
        let data_bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) };
        assert!(data_bytes.len() as vk::DeviceSize <= self.size);

        self.map(allocator);
        unsafe {
            std::ptr::copy_nonoverlapping(data_bytes.as_ptr(), self.map_ptr.unwrap(), size_of_val(data));
        }
        allocator.flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize).unwrap();
        self.unmap(allocator);
    }

    /// map 和 unmap 需要匹配
    #[inline]
    pub fn map(&mut self, allocator: &MemAllocator) {
        if self.map_ptr.is_some() {
            return;
        }
        unsafe {
            self.map_ptr = Some(allocator.map_memory(&mut self.allocation).unwrap());
        }
    }

    #[inline]
    pub fn flush(&mut self, allocator: &MemAllocator, offset: vk::DeviceSize, size: vk::DeviceSize) {
        allocator.flush_allocation(&self.allocation, offset, size).unwrap();
    }

    #[inline]
    pub fn unmap(&mut self, allocator: &MemAllocator) {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            allocator.unmap_memory(&mut self.allocation);
            self.map_ptr = None;
        }
    }
}

impl Drop for ManagedBuffer {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        debug_assert!(self.destroyed, "ManagedBuffer must be destroyed before being dropped.");
    }
}
