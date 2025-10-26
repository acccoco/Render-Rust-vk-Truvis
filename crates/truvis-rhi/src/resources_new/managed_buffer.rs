use ash::vk;
use vk_mem::Alloc;

use crate::render_context::RenderContext;

pub struct Buffer2 {
    vk_handle: vk::Buffer,
    allocation: vk_mem::Allocation,

    size: vk::DeviceSize,

    map_ptr: Option<*mut u8>,
    device_addr: Option<vk::DeviceAddress>,

    debug_name: String,

    #[cfg(debug_assertions)]
    destroyed: bool,
}
// create & destroy
impl Buffer2 {
    /// # Note
    /// - align: 其实地址的内存对齐，默认对齐到 8 字节
    /// - 优先使用 device memory
    #[deprecated]
    pub fn new(
        buffer_size: vk::DeviceSize,
        buffer_usage: vk::BufferUsageFlags,
        align: Option<vk::DeviceSize>,
        mem_map: bool,
        name: impl AsRef<str>,
    ) -> Self {
        let buffer_ci = vk::BufferCreateInfo::default().size(buffer_size).usage(buffer_usage);
        let alloc_ci = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            flags: if mem_map {
                vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
            } else {
                vk_mem::AllocationCreateFlags::empty()
            },
            ..Default::default()
        };

        let align = align.unwrap_or(8);
        let (buffer, alloc) = unsafe {
            RenderContext::get().allocator.create_buffer_with_alignment(&buffer_ci, &alloc_ci, align).unwrap()
        };

        let mut device_addr = None;
        if buffer_usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
            let device_functions = RenderContext::get().device_functions();
            unsafe {
                device_addr = Some(
                    device_functions.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(buffer)),
                );
            }
        }

        RenderContext::get().device_functions().set_object_debug_name(buffer, format!("Buffer::{}", name.as_ref()));
        Self {
            vk_handle: buffer,
            allocation: alloc,
            size: buffer_size,
            map_ptr: None,
            device_addr,
            debug_name: name.as_ref().to_string(),

            #[cfg(debug_assertions)]
            destroyed: false,
        }
    }

    #[inline]
    pub fn new_stage_buffer(buffer_size: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        Self::new(buffer_size, vk::BufferUsageFlags::TRANSFER_SRC, None, true, name)
    }

    pub fn destroy(mut self) {
        unsafe {
            RenderContext::get().allocator().destroy_buffer(self.vk_handle, &mut self.allocation);
        }
        self.destroyed = true;
    }
}
// getter
impl Buffer2 {
    #[inline]
    pub fn vk_buffer(&self) -> vk::Buffer {
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
    pub fn device_address(&self) -> vk::DeviceAddress {
        self.device_addr.unwrap()
    }
}
// tools
impl Buffer2 {
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
        stage_buffer.transfer_data_by_mmap(data);

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
    pub fn transfer_data_by_mmap<T>(&mut self, data: &[T])
    where
        T: Sized + Copy,
    {
        // 准备好源数据
        let data_bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) };
        assert!(data_bytes.len() as vk::DeviceSize <= self.size);

        self.map();
        unsafe {
            std::ptr::copy_nonoverlapping(data_bytes.as_ptr(), self.map_ptr.unwrap(), size_of_val(data));
        }
        RenderContext::get()
            .allocator()
            .flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize)
            .unwrap();
        self.unmap();
    }

    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到
    /// self
    ///
    /// sync 表示这个函数是同步等待的，会阻塞运行
    ///
    /// # Note
    /// * 避免使用这个将 *小块* 数据从内存传到 GPU，推荐使用 cmd transfer
    /// * 这个应该是用来传输大块数据的
    pub fn transfer_data_sync2(&mut self, total_size: vk::DeviceSize, do_with_stage_buffer: impl FnOnce(&mut Buffer2)) {
        let mut stage_buffer = Self::new_stage_buffer(total_size, format!("{}-stage-buffer", self.debug_name));

        do_with_stage_buffer(&mut stage_buffer);

        let cmd_name = format!("{}-transfer-data", &self.debug_name);
        RenderContext::get().one_time_exec(
            |cmd| {
                cmd.cmd_copy_buffer(
                    &stage_buffer,
                    self,
                    &[vk::BufferCopy {
                        size: total_size,
                        ..Default::default()
                    }],
                );
            },
            &cmd_name,
        );
    }

    /// map 和 unmap 需要匹配
    #[inline]
    pub fn map(&mut self) {
        if self.map_ptr.is_some() {
            return;
        }
        unsafe {
            self.map_ptr = Some(RenderContext::get().allocator().map_memory(&mut self.allocation).unwrap());
        }
    }

    #[inline]
    pub fn flush(&mut self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        RenderContext::get().allocator().flush_allocation(&self.allocation, offset, size).unwrap();
    }

    #[inline]
    pub fn unmap(&mut self) {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            RenderContext::get().allocator().unmap_memory(&mut self.allocation);
            self.map_ptr = None;
        }
    }
}

impl Drop for Buffer2 {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        debug_assert!(self.destroyed, "ManagedBuffer must be destroyed before being dropped.");
    }
}
