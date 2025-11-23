use ash::{vk, vk::Handle};

use crate::{
    foundation::debug_messenger::DebugType,
    gfx::Gfx,
    resources::{
        handles::{BufferHandle, IndexBufferHandle},
        layout::GfxIndexType,
        resource_data::BufferType,
    },
};

/// 顶点类型是 u32
pub struct GfxIndexBuffer<T: GfxIndexType> {
    handle: IndexBufferHandle,

    /// 索引数量
    index_cnt: usize,

    _phantom: std::marker::PhantomData<T>,
}

// init & destroy
impl<T: GfxIndexType> GfxIndexBuffer<T> {
    pub fn new(index_cnt: usize, debug_name: impl AsRef<str>) -> Self {
        let size = index_cnt * std::mem::size_of::<T>();
        let mut rm = Gfx::get().resource_manager();
        let handle = rm.create_buffer(
            size as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            false,
            BufferType::Index,
            debug_name.as_ref(),
        );

        Self {
            handle: IndexBufferHandle { inner: handle.inner },
            index_cnt,
            _phantom: std::marker::PhantomData,
        }
    }

    /// 创建 index buffer，并向其内写入数据
    #[inline]
    pub fn new_with_data(data: &[T], debug_name: impl AsRef<str>) -> Self
    where
        T: bytemuck::Pod,
    {
        let index_buffer = Self::new(data.len(), debug_name);
        index_buffer.transfer_data_sync(data);
        index_buffer
    }

    pub fn transfer_data_sync(&self, data: &[T])
    where
        T: bytemuck::Pod,
    {
        let size_bytes = std::mem::size_of_val(data);
        let mut rm = Gfx::get().resource_manager();

        // Create staging buffer
        let staging_handle = rm.create_buffer(
            size_bytes as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            "index-staging",
        );

        // Copy to staging
        {
            let buffer_res = rm.get_buffer_mut(staging_handle).unwrap();
            if let Some(ptr) = buffer_res.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, ptr, size_bytes);
                    // Flush
                    let allocator = Gfx::get().allocator();
                    allocator.flush_allocation(&buffer_res.allocation, 0, size_bytes as vk::DeviceSize).unwrap();
                }
            }
        }

        let staging_vk = rm.get_buffer(staging_handle).unwrap().buffer;
        let dst_vk = self.vk_buffer();

        // Copy command
        Gfx::get().one_time_exec(
            |cmd| {
                let region = vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size: size_bytes as vk::DeviceSize,
                };
                unsafe {
                    Gfx::get().gfx_device().cmd_copy_buffer(cmd.vk_handle(), staging_vk, dst_vk, &[region]);
                }
            },
            "upload-index-buffer",
        );

        // Destroy staging
        rm.destroy_buffer_immediate(staging_handle);
    }
}
// getter
impl<T: GfxIndexType> GfxIndexBuffer<T> {
    #[inline]
    pub fn index_type() -> vk::IndexType {
        T::VK_INDEX_TYPE
    }

    #[inline]
    pub fn index_cnt(&self) -> usize {
        self.index_cnt
    }

    #[inline]
    pub fn vk_buffer(&self) -> vk::Buffer {
        let rm = Gfx::get().resource_manager();
        let handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.get_buffer(handle).unwrap().buffer
    }

    #[inline]
    pub fn device_address(&self) -> vk::DeviceAddress {
        let rm = Gfx::get().resource_manager();
        let handle = BufferHandle {
            inner: self.handle.inner,
        };
        rm.get_buffer(handle).unwrap().device_addr.unwrap_or(0)
    }
}

impl<T: GfxIndexType> DebugType for GfxIndexBuffer<T> {
    fn debug_type_name() -> &'static str {
        "IndexBuffer"
    }

    fn vk_handle(&self) -> impl Handle {
        self.vk_buffer()
    }
}

pub type GfxIndex32Buffer = GfxIndexBuffer<u32>;
pub type GfxIndex16Buffer = GfxIndexBuffer<u16>;
