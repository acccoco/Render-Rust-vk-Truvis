use std::{ffi::c_void, rc::Rc};

use ash::vk;
use vk_mem::Alloc;

use crate::{
    foundation::debug_messenger::DebugType, render_context::RenderContext, resources::buffer_creator::BufferCreateInfo,
};

pub struct Buffer {
    pub handle: vk::Buffer,
    pub allocation: vk_mem::Allocation,

    pub map_ptr: Option<*mut u8>,
    pub size: vk::DeviceSize,

    pub debug_name: String,

    pub device_addr: Option<vk::DeviceAddress>,

    _buffer_info: Rc<BufferCreateInfo>,
    _alloc_info: Rc<vk_mem::AllocationCreateInfo>,
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
            allocator.destroy_buffer(self.handle, &mut self.allocation);
        }
    }
}

// constructor & getter & builder
impl Buffer {
    /// # param
    /// * align: 当 buffer 处于一个大的 memory block 中时，align 用来指定 buffer 的起始 offset
    pub fn new(
        buffer_ci: Rc<BufferCreateInfo>,
        alloc_ci: Rc<vk_mem::AllocationCreateInfo>,
        align: Option<vk::DeviceSize>,
        debug_name: impl AsRef<str>,
    ) -> Self {
        let device_functions = RenderContext::get().device_functions();
        let allocator = RenderContext::get().allocator();
        unsafe {
            // 默认给 8 的 align，表示所有的 buffer 其实地址一定会和 8 对齐
            let (buffer, allocation) =
                allocator.create_buffer_with_alignment(buffer_ci.info(), &alloc_ci, align.unwrap_or(8)).unwrap();

            let buffer = Self {
                handle: buffer,
                allocation,
                map_ptr: None,
                size: buffer_ci.size(),
                debug_name: debug_name.as_ref().to_string(),
                _buffer_info: buffer_ci,
                _alloc_info: alloc_ci,
                device_addr: None,
            };
            device_functions.set_debug_name(&buffer, &buffer.debug_name);
            buffer
        }
    }

    #[inline]
    pub fn new_device_buffer(size: vk::DeviceSize, flags: vk::BufferUsageFlags, debug_name: impl AsRef<str>) -> Self {
        Self::new(
            Rc::new(BufferCreateInfo::new(size, flags)),
            Rc::new(vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            }),
            None,
            debug_name,
        )
    }

    #[inline]
    pub fn new_acceleration_instance_buffer(size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self::new_device_buffer(
            size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::TRANSFER_DST,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_stage_buffer(size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self::new(
            Rc::new(BufferCreateInfo::new(size, vk::BufferUsageFlags::TRANSFER_SRC)),
            Rc::new(vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::Auto,
                flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                ..Default::default()
            }),
            None,
            debug_name,
        )
    }

    #[inline]
    pub fn new_accleration_buffer(size: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new_device_buffer(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name,
        )
    }

    #[inline]
    pub fn new_accleration_scratch_buffer(size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self::new_device_buffer(
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name,
        )
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::Buffer {
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
        self.map_ptr.unwrap_or_else(|| {
            panic!("Buffer is not mapped, please call map() before using mapped_ptr()");
        })
    }

    #[inline]
    pub fn map(&mut self) {
        if self.map_ptr.is_some() {
            return;
        }
        unsafe {
            let allocator = RenderContext::get().allocator();
            self.map_ptr = Some(allocator.map_memory(&mut self.allocation).unwrap());
        }
    }

    #[inline]
    pub fn flush(&mut self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        let allocator = RenderContext::get().allocator();
        allocator.flush_allocation(&self.allocation, offset, size).unwrap();
    }

    #[inline]
    pub fn unmap(&mut self) {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            let allocator = RenderContext::get().allocator();
            allocator.unmap_memory(&mut self.allocation);
            self.map_ptr = None;
        }
    }

    /// 通过 mem map 的方式将 data 传入到 buffer 中
    ///
    /// 注：确保 buffer 内存的对齐方式和 T 保持一致
    pub fn transfer_data_by_mem_map<T>(&mut self, data: &[T])
    where
        T: Sized + Copy,
    {
        self.map();
        unsafe {
            // 这里的 size 是 buffer 的最大 size
            // 这个函数主要处理的是 device 的内存对齐(std140, std430)和 host
            // 的内存对齐不一致的问题 这个函数会自动的为数据增加 padding
            // 由于已经手动为 struct 添加了 padding，因此这个函数暂时用不上
            let mut slice =
                ash::util::Align::new(self.map_ptr.unwrap() as *mut c_void, align_of::<T>() as u64, self.size);
            slice.copy_from_slice(data);
            let allocator = RenderContext::get().allocator();
            allocator.flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize).unwrap();
        }
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
    pub fn copy_from_sync(&mut self, data: &[impl Sized + Copy]) {
        let mut stage_buffer =
            Self::new_stage_buffer(size_of_val(data) as vk::DeviceSize, format!("{}-stage-buffer", self.debug_name));

        stage_buffer.transfer_data_by_mem_map(data);

        let cmd_name = format!("{}-transfer-data", &self.debug_name);
        RenderContext::get().one_time_exec(
            |cmd| {
                cmd.cmd_copy_buffer_1(
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
}
