use std::{ffi::c_void, rc::Rc};

use ash::vk;
use vk_mem::Alloc;

use crate::framework::{
    core::{allocator::RhiAllocator, command_buffer::RhiCommandBuffer, device::RhiDevice},
    render_core::Rhi,
};


pub struct RhiBufferCreateInfo
{
    inner: vk::BufferCreateInfo<'static>,
    queue_family_indices: Vec<u32>,
}

impl RhiBufferCreateInfo
{
    #[inline]
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self
    {
        Self {
            inner: vk::BufferCreateInfo {
                size,
                usage,
                ..Default::default()
            },
            queue_family_indices: Vec::new(),
        }
    }

    #[inline]
    pub fn info(&self) -> &vk::BufferCreateInfo
    {
        &self.inner
    }

    #[inline]
    pub fn size(&self) -> vk::DeviceSize
    {
        self.inner.size
    }

    #[inline]
    pub fn queue_family_indices(mut self, indices: &[u32]) -> Self
    {
        self.queue_family_indices = indices.to_vec();

        self.inner.queue_family_index_count = indices.len() as u32;
        self.inner.p_queue_family_indices = self.queue_family_indices.as_ptr();

        self
    }
}


pub struct RhiBuffer
{
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,

    map_ptr: Option<*mut u8>,
    size: vk::DeviceSize,

    debug_name: String,

    allocator: Rc<RhiAllocator>,
    device: Rc<RhiDevice>,

    buffer_info: Rc<RhiBufferCreateInfo>,
    alloc_info: Rc<vk_mem::AllocationCreateInfo>,
}

impl RhiBuffer
{
    /// # param
    /// * align: 对 memory 的 offset align 限制
    pub fn new(
        rhi: &Rhi,
        buffer_ci: Rc<RhiBufferCreateInfo>,
        alloc_ci: Rc<vk_mem::AllocationCreateInfo>,
        align: Option<vk::DeviceSize>,
        debug_name: &str,
    ) -> Self
    {
        unsafe {
            let (buffer, allocation) = if let Some(offset_align) = align {
                rhi.allocator.create_buffer_with_alignment(buffer_ci.info(), &alloc_ci, offset_align).unwrap()
            } else {
                rhi.allocator.create_buffer(buffer_ci.info(), &alloc_ci).unwrap()
            };

            rhi.set_debug_name(buffer, debug_name);
            Self {
                handle: buffer,
                allocation,
                map_ptr: None,
                size: buffer_ci.size(),
                debug_name: debug_name.to_string(),
                allocator: rhi.allocator.clone(),
                device: rhi.device.clone(),
                buffer_info: buffer_ci,
                alloc_info: alloc_ci,
            }
        }
    }

    #[inline]
    pub fn new_device_buffer(rhi: &Rhi, size: vk::DeviceSize, flags: vk::BufferUsageFlags, debug_name: &str) -> Self
    {
        Self::new(
            rhi,
            Rc::new(RhiBufferCreateInfo::new(size, flags)),
            Rc::new(vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            }),
            None,
            debug_name,
        )
    }

    #[inline]
    pub fn new_acceleration_instance_buffer<S>(rhi: &Rhi, size: vk::DeviceSize, debug_name: S) -> Self
    where
        S: AsRef<str>,
    {
        Self::new_device_buffer(
            rhi,
            size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS |
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR |
                vk::BufferUsageFlags::TRANSFER_DST,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_stage_buffer(rhi: &Rhi, size: vk::DeviceSize, debug_name: &str) -> Self
    {
        Self::new(
            rhi,
            Rc::new(RhiBufferCreateInfo::new(size, vk::BufferUsageFlags::TRANSFER_SRC)),
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
    pub fn new_index_buffer(rhi: &Rhi, size: usize, debug_name: &str) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER |
                vk::BufferUsageFlags::TRANSFER_DST |
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS |
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_vertex_buffer(rhi: &Rhi, size: usize, debug_name: &str) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER |
                vk::BufferUsageFlags::TRANSFER_DST |
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS |
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_accleration_buffer<S: AsRef<str>>(rhi: &Rhi, size: usize, debug_name: S) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_accleration_scratch_buffer<S: AsRef<str>>(rhi: &Rhi, size: vk::DeviceSize, debug_name: S) -> Self
    {
        Self::new_device_buffer(
            rhi,
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name.as_ref(),
        )
    }

    /// getter
    #[inline]
    pub fn handle(&self) -> vk::Buffer
    {
        self.handle
    }

    #[inline]
    pub fn map(&mut self)
    {
        if self.map_ptr.is_some() {
            return;
        }
        unsafe {
            self.map_ptr = Some(self.allocator.map_memory(&mut self.allocation).unwrap());
        }
    }

    #[inline]
    pub fn unmap(&mut self)
    {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            self.allocator.unmap_memory(&mut self.allocation);
            self.map_ptr = None;
        }
    }

    #[inline]
    pub fn destroy(mut self)
    {
        unsafe {
            self.allocator.destroy_buffer(self.handle, &mut self.allocation);
        }
    }


    /// 通过 mem map 的方式将 data 传入到 buffer 中
    pub fn transfer_data_by_mem_map<T>(&mut self, data: &[T])
    where
        T: Sized + Copy,
    {
        self.map();
        unsafe {
            // 这里的 size 是目标内存的最大 size
            // align 表示目标内存位置额外的内存对齐要求，这里使用 align_of 表示和 rust 中 [T; n] 的保持一致
            let mut slice =
                ash::util::Align::new(self.map_ptr.unwrap() as *mut c_void, align_of::<T>() as u64, self.size);
            slice.copy_from_slice(data);
            self.allocator.flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize).unwrap();
        }
        self.unmap();
    }

    // FIXME 这个隐含同步等待，需要特殊标注一下
    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到 self
    ///
    /// # Note
    /// * 避免使用这个将 *小块* 数据从内存传到 GPU，推荐使用 cmd transfer
    /// * 这个应该是用来传输大块数据的
    pub fn transfer_data_by_stage_buffer<T>(&mut self, rhi: &Rhi, data: &[T])
    where
        T: Sized + Copy,
    {
        let mut stage_buffer = Self::new_stage_buffer(
            rhi,
            size_of_val(data) as vk::DeviceSize,
            &format!("{}-stage-buffer", self.debug_name),
        );

        stage_buffer.transfer_data_by_mem_map(data);

        let cmd_name = format!("{}-transfer-data", &self.debug_name);
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.transfer_command_pool.clone(),
            &rhi.transfer_queue,
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

        stage_buffer.destroy();
    }

    pub fn get_device_address(&self) -> vk::DeviceAddress
    {
        unsafe { self.device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(self.handle)) }
    }
}
