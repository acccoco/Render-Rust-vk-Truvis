use ash::vk;
use std::ptr;

use vk_mem::Alloc;

use crate::{foundation::debug_messenger::DebugType, render_context::RenderContext};

pub struct Buffer {
    pub handle: vk::Buffer,
    pub allocation: vk_mem::Allocation,

    pub map_ptr: Option<*mut u8>,
    pub size: vk::DeviceSize,

    pub device_addr: Option<vk::DeviceAddress>,

    #[cfg(debug_assertions)]
    pub debug_name: String,
}

impl DebugType for Buffer {
    fn debug_type_name() -> &'static str {
        "RhiBuffer"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let allocator = RenderContext::get().allocator();
        unsafe {
            if self.map_ptr.is_some() {
                allocator.unmap_memory(&mut self.allocation);
            }

            allocator.destroy_buffer(self.handle, &mut self.allocation);
        }
    }
}

// init & destroy
impl Buffer {
    /// - align: 当 buffer 处于一个大的 memory block 中时，align 用来指定 buffer 的起始 offset,
    /// 其实地址的内存对齐，默认对齐到 8 字节
    /// - 优先使用 device memory
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
        let (buffer, mut alloc) = unsafe {
            RenderContext::get().allocator.create_buffer_with_alignment(&buffer_ci, &alloc_ci, align).unwrap()
        };

        let mut mapped_ptr = None;
        if mem_map {
            unsafe {
                let allocator = RenderContext::get().allocator();
                mapped_ptr = Some(allocator.map_memory(&mut alloc).unwrap());
            }
        }

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
            handle: buffer,
            allocation: alloc,
            size: buffer_size,
            map_ptr: mapped_ptr,
            device_addr,
            debug_name: name.as_ref().to_string(),
        }
    }

    #[inline]
    pub fn new_stage_buffer(size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self::new(size, vk::BufferUsageFlags::TRANSFER_SRC, None, true, debug_name)
    }
}

// getter
impl Buffer {
    #[inline]
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.handle
    }

    #[inline]
    pub fn device_address(&self) -> vk::DeviceAddress {
        self.device_addr.unwrap_or_else(|| {
            let device_functions = RenderContext::get().device_functions();
            unsafe {
                device_functions.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(self.handle))
            }
        })
    }

    #[inline]
    pub fn size(&self) -> vk::DeviceSize {
        self.size
    }
}

// tools
impl Buffer {
    #[inline]
    pub fn mapped_ptr(&self) -> *mut u8 {
        self.map_ptr.expect("Buffer is not mapped, please call map() before using mapped_ptr()")
    }

    #[inline]
    pub fn flush(&self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        let allocator = RenderContext::get().allocator();
        allocator.flush_allocation(&self.allocation, offset, size).unwrap();
    }

    /// 通过 mem map 的方式将 data 传入到 buffer 中
    pub fn transfer_data_by_mmap<T>(&mut self, data: &[T])
    where
        T: Sized + Copy,
    {
        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr() as *const u8, self.mapped_ptr(), size_of_val(data));

            let allocator = RenderContext::get().allocator();
            allocator.flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize).unwrap();
        }
    }

    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到
    /// self
    ///
    /// sync 表示这个函数是同步等待的，会阻塞运行
    ///
    /// # Note
    /// * 避免使用这个将 *小块* 数据从内存传到 GPU，推荐使用 cmd transfer
    /// * 这个应该是用来传输大块数据的
    pub fn transfer_data_sync(&mut self, data: &[impl Sized + Copy]) {
        let mut stage_buffer =
            Self::new_stage_buffer(size_of_val(data) as vk::DeviceSize, format!("{}-stage-buffer", self.debug_name));

        stage_buffer.transfer_data_by_mmap(data);

        let cmd_name = format!("{}-transfer-data", &self.debug_name);
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
            &cmd_name,
        );
    }

    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到
    /// self
    ///
    /// sync 表示这个函数是同步等待的，会阻塞运行
    ///
    /// # Note
    /// * 避免使用这个将 *小块* 数据从内存传到 GPU，推荐使用 cmd transfer
    /// * 这个应该是用来传输大块数据的
    pub fn transfer_data_sync2(&mut self, total_size: vk::DeviceSize, do_with_stage_buffer: impl FnOnce(&mut Buffer)) {
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
}
