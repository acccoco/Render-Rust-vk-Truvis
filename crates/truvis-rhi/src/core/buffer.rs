use ash::vk;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::{ffi::c_void, rc::Rc};
use vk_mem::Alloc;

use crate::core::debug_utils::RhiDebugType;
use crate::{
    core::{allocator::RhiAllocator, command_buffer::RhiCommandBuffer, device::RhiDevice},
    rhi::Rhi,
};

/// 定义一个 macro，自动为各种派生 Buffer 类型实现 Deref、DerefMut 和 RhiDebugType
macro_rules! impl_derive_buffer {
    // 支持泛型的版本
    ($name:ident<$($generic:ident $(: $bound:path)?),*>, $target:ty, $inner:ident) => {
        impl<$($generic $(: $bound)?),*> Deref for $name<$($generic),*> {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$inner
            }
        }

        impl<$($generic $(: $bound)?),*> DerefMut for $name<$($generic),*> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$inner
            }
        }

        impl<$($generic $(: $bound)?),*> RhiDebugType for $name<$($generic),*> {
            fn debug_type_name() -> &'static str {
                stringify!($name)
            }

            fn vk_handle(&self) -> impl vk::Handle {
                self.$inner.vk_handle()
            }
        }
    };
    // 非泛型版本
    ($name:ident, $target:ty, $inner:ident) => {
        impl Deref for $name {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$inner
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$inner
            }
        }

        impl RhiDebugType for $name {
            fn debug_type_name() -> &'static str {
                stringify!($name)
            }

            fn vk_handle(&self) -> impl vk::Handle {
                self.$inner.vk_handle()
            }
        }
    };
}

pub struct RhiBufferCreateInfo {
    inner: vk::BufferCreateInfo<'static>,
    queue_family_indices: Vec<u32>,
}
impl RhiBufferCreateInfo {
    #[inline]
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
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
    pub fn info(&self) -> &vk::BufferCreateInfo {
        &self.inner
    }

    #[inline]
    pub fn size(&self) -> vk::DeviceSize {
        self.inner.size
    }

    #[inline]
    pub fn queue_family_indices(mut self, indices: &[u32]) -> Self {
        self.queue_family_indices = indices.to_vec();

        self.inner.queue_family_index_count = indices.len() as u32;
        self.inner.p_queue_family_indices = self.queue_family_indices.as_ptr();

        self
    }
}

pub struct RhiBuffer {
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,

    map_ptr: Option<*mut u8>,
    size: vk::DeviceSize,

    debug_name: String,

    allocator: Rc<RhiAllocator>,
    device: Rc<RhiDevice>,

    device_addr: Option<vk::DeviceAddress>,

    _buffer_info: Rc<RhiBufferCreateInfo>,
    _alloc_info: Rc<vk_mem::AllocationCreateInfo>,
}
impl RhiDebugType for RhiBuffer {
    fn debug_type_name() -> &'static str {
        "RhiBuffer"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
impl Drop for RhiBuffer {
    fn drop(&mut self) {
        unsafe {
            self.allocator.destroy_buffer(self.handle, &mut self.allocation);
        }
    }
}
// constructor & getter & builder
impl RhiBuffer {
    /// # param
    /// * align: 当 buffer 处于一个大的 memory block 中时，align 用来指定 buffer 的起始 offset
    pub fn new(
        rhi: &Rhi,
        buffer_ci: Rc<RhiBufferCreateInfo>,
        alloc_ci: Rc<vk_mem::AllocationCreateInfo>,
        align: Option<vk::DeviceSize>,
        debug_name: impl AsRef<str>,
    ) -> Self {
        unsafe {
            // 默认给 8 的 align，表示所有的 buffer 其实地址一定会和 8 对齐
            let (buffer, allocation) =
                rhi.allocator.create_buffer_with_alignment(buffer_ci.info(), &alloc_ci, align.unwrap_or(8)).unwrap();

            let buffer = Self {
                handle: buffer,
                allocation,
                map_ptr: None,
                size: buffer_ci.size(),
                debug_name: debug_name.as_ref().to_string(),
                allocator: rhi.allocator.clone(),
                device: rhi.device.clone(),
                _buffer_info: buffer_ci,
                _alloc_info: alloc_ci,
                device_addr: None,
            };
            rhi.device.debug_utils().set_debug_name(&buffer, &buffer.debug_name);
            buffer
        }
    }

    #[inline]
    pub fn new_device_buffer(
        rhi: &Rhi,
        size: vk::DeviceSize,
        flags: vk::BufferUsageFlags,
        debug_name: impl AsRef<str>,
    ) -> Self {
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
    pub fn new_acceleration_instance_buffer(rhi: &Rhi, size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self::new_device_buffer(
            rhi,
            size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::TRANSFER_DST,
            debug_name.as_ref(),
        )
    }

    #[inline]
    pub fn new_stage_buffer(rhi: &Rhi, size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
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
    pub fn new_accleration_buffer(rhi: &Rhi, size: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            debug_name,
        )
    }

    #[inline]
    pub fn new_accleration_scratch_buffer(rhi: &Rhi, size: vk::DeviceSize, debug_name: impl AsRef<str>) -> Self {
        Self::new_device_buffer(
            rhi,
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
        self.device_addr.unwrap_or_else(|| unsafe {
            self.device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(self.handle))
        })
    }

    #[inline]
    pub fn size(&self) -> vk::DeviceSize {
        self.size
    }
}
// tools
impl RhiBuffer {
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
            self.map_ptr = Some(self.allocator.map_memory(&mut self.allocation).unwrap());
        }
    }

    #[inline]
    pub fn flush(&mut self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        self.allocator.flush_allocation(&self.allocation, offset, size).unwrap();
    }

    #[inline]
    pub fn unmap(&mut self) {
        if self.map_ptr.is_none() {
            return;
        }
        unsafe {
            self.allocator.unmap_memory(&mut self.allocation);
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
            // 这个函数主要处理的是 device 的内存对齐(std140, std430)和 host 的内存对齐不一致的问题
            // 这个函数会自动的为数据增加 padding
            // 由于已经手动为 struct 添加了 padding，因此这个函数暂时用不上
            let mut slice =
                ash::util::Align::new(self.map_ptr.unwrap() as *mut c_void, align_of::<T>() as u64, self.size);
            slice.copy_from_slice(data);
            self.allocator.flush_allocation(&self.allocation, 0, size_of_val(data) as vk::DeviceSize).unwrap();
        }
        self.unmap();
    }

    /// 创建一个临时的 stage buffer，先将数据放入 stage buffer，再 transfer 到 self
    ///
    /// sync 表示这个函数是同步等待的，会阻塞运行
    ///
    /// # Note
    /// * 避免使用这个将 *小块* 数据从内存传到 GPU，推荐使用 cmd transfer
    /// * 这个应该是用来传输大块数据的
    pub fn transfer_data_sync(&mut self, rhi: &Rhi, data: &[impl Sized + Copy]) {
        let mut stage_buffer = Self::new_stage_buffer(
            rhi,
            size_of_val(data) as vk::DeviceSize,
            format!("{}-stage-buffer", self.debug_name),
        );

        stage_buffer.transfer_data_by_mem_map(data);

        let cmd_name = format!("{}-transfer-data", &self.debug_name);
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.temp_graphics_command_pool.clone(),
            &rhi.graphics_queue,
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

    /// 默认的 descriptor buffer info
    #[inline]
    pub fn get_descriptor_buffer_info_ubo<T: Sized>(&self) -> vk::DescriptorBufferInfo {
        vk::DescriptorBufferInfo::default().buffer(self.handle).offset(0).range(size_of::<T>() as vk::DeviceSize)
    }
}

/// buffer 内存放的是结构体或者结构体的数组
pub struct RhiStructuredBuffer<T: bytemuck::Pod> {
    inner: RhiBuffer,
    /// 结构体的数量
    len: usize,
    _phantom: PhantomData<T>,
}
impl_derive_buffer!(RhiStructuredBuffer<T: bytemuck::Pod>, RhiBuffer, inner);
impl<T: bytemuck::Pod> RhiStructuredBuffer<T> {
    #[inline]
    pub fn new_ubo(rhi: &Rhi, len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(
            rhi,
            debug_name,
            len,
            vk::BufferUsageFlags::UNIFORM_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            false,
        )
    }

    #[inline]
    pub fn new_stage_buffer(rhi: &Rhi, len: usize, debug_name: impl AsRef<str>) -> Self {
        Self::new(rhi, debug_name, len, vk::BufferUsageFlags::TRANSFER_SRC, true)
    }

    #[inline]
    pub fn new(
        rhi: &Rhi,
        debug_name: impl AsRef<str>,
        len: usize,
        buffer_usage_flags: vk::BufferUsageFlags,
        mapped: bool,
    ) -> Self {
        let allocation_create_flags = if mapped {
            // TODO 或许可以优化这个 flag
            vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
        } else {
            vk_mem::AllocationCreateFlags::empty()
        };

        Self {
            inner: RhiBuffer::new(
                rhi,
                Rc::new(RhiBufferCreateInfo::new((len * size_of::<T>()) as vk::DeviceSize, buffer_usage_flags)),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: allocation_create_flags,
                    ..Default::default()
                }),
                // TODO 可能不需要这个 align
                Some(rhi.device.min_ubo_offset_align()),
                debug_name,
            ),
            len,
            _phantom: PhantomData,
        }
    }

    pub fn mapped_slice(&mut self) -> &mut [T] {
        let mapped_ptr = self.inner.mapped_ptr();
        unsafe { std::slice::from_raw_parts_mut(mapped_ptr as *mut T, self.len) }
    }
}

pub struct RhiStageBuffer<T: bytemuck::Pod> {
    inner: RhiBuffer,
    _phantom: PhantomData<T>,
}
impl_derive_buffer!(RhiStageBuffer<T: bytemuck::Pod>, RhiBuffer, inner);
impl<T: bytemuck::Pod> RhiStageBuffer<T> {
    pub fn new(rhi: &Rhi, debug_name: impl AsRef<str>) -> Self {
        let buffer = Self {
            inner: RhiBuffer::new(
                rhi,
                Rc::new(RhiBufferCreateInfo::new(size_of::<T>() as vk::DeviceSize, vk::BufferUsageFlags::TRANSFER_SRC)),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                    ..Default::default()
                }),
                None,
                debug_name,
            ),
            _phantom: PhantomData,
        };
        rhi.device.debug_utils().set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    // BUG 可能需要考虑内存对齐
    pub fn transfer(&mut self, trans_func: &dyn Fn(&mut T)) {
        self.inner.map();
        unsafe {
            let ptr = self.inner.map_ptr.unwrap() as *mut T;

            trans_func(&mut *ptr);
        }
        self.inner.allocator.flush_allocation(&self.inner.allocation, 0, size_of::<T>() as vk::DeviceSize).unwrap();
        self.inner.unmap();
    }
}

pub struct RhiSBTBuffer {
    _inner: RhiBuffer,
}
impl_derive_buffer!(RhiSBTBuffer, RhiBuffer, _inner);
impl RhiSBTBuffer {
    pub fn new(rhi: &Rhi, size: vk::DeviceSize, align: vk::DeviceSize, name: impl AsRef<str>) -> Self {
        let buffer = Self {
            _inner: RhiBuffer::new(
                rhi,
                Rc::new(RhiBufferCreateInfo::new(
                    size,
                    vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                        | vk::BufferUsageFlags::TRANSFER_SRC
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                )),
                Rc::new(vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
                    ..Default::default()
                }),
                Some(align),
                format!("SBTBuffer::{}", name.as_ref()),
            ),
        };
        rhi.device.debug_utils().set_debug_name(&buffer, name.as_ref());
        buffer
    }

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self._inner.handle
    }
}

/// 顶点类型是 u32
pub struct RhiIndexBuffer {
    inner: RhiBuffer,

    /// 索引数量
    index_cnt: usize,
}
impl_derive_buffer!(RhiIndexBuffer, RhiBuffer, inner);
impl RhiIndexBuffer {
    pub fn new(rhi: &Rhi, index_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = index_cnt * size_of::<u32>();
        let buffer = RhiBuffer::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name,
        );

        let buffer = Self {
            inner: buffer,
            index_cnt,
        };
        rhi.device.debug_utils().set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(rhi: &Rhi, data: &[u32], debug_name: impl AsRef<str>) -> Self {
        let mut index_buffer = Self::new(rhi, data.len(), debug_name);
        index_buffer.transfer_data_sync(rhi, data);
        index_buffer
    }

    #[inline]
    pub fn index_type() -> vk::IndexType {
        vk::IndexType::UINT32
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.index_cnt
    }
}

pub struct RhiVertexBuffer<V: Sized> {
    inner: RhiBuffer,

    /// 顶点数量
    vertex_cnt: usize,

    _phantom: PhantomData<V>,
}
impl_derive_buffer!(RhiVertexBuffer<V: Sized>, RhiBuffer, inner);
impl<V: Sized> RhiVertexBuffer<V> {
    pub fn new(rhi: &Rhi, vertex_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = vertex_cnt * size_of::<V>();
        let buffer = RhiBuffer::new_device_buffer(
            rhi,
            size as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            debug_name,
        );

        let buffer = Self {
            inner: buffer,
            vertex_cnt,
            _phantom: PhantomData,
        };
        rhi.device.debug_utils().set_debug_name(&buffer, &buffer.inner.debug_name);
        buffer
    }

    #[inline]
    pub fn vertex_cnt(&self) -> usize {
        self.vertex_cnt
    }
}
