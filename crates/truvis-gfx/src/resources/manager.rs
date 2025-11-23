use std::marker::PhantomData;

use ash::vk;
use slotmap::SlotMap;
use vk_mem::Alloc;

use crate::commands::barrier::GfxImageBarrier;
use crate::gfx::Gfx;

use super::{
    handles::{
        BufferHandle, ImageHandle, ImageViewHandle, IndexBufferHandle, InnerBufferHandle, InnerImageHandle,
        InnerImageViewHandle, StructuredBufferHandle, VertexBufferHandle,
    },
    layout::{GfxIndexType, GfxVertexLayout},
    resource_data::{BufferResource, BufferType, ImageResource, ImageSource, ImageViewResource},
};

/// 资源管理器
///
/// 负责管理所有的 GPU 资源，包括 Buffer、Image 和 ImageView。
/// 使用 SlotMap 存储资源，对外提供轻量级的 Handle。
/// 支持资源的延迟销毁（Frames in Flight）。
pub struct ResourceManager {
    /// 存储所有的 Buffer 资源
    buffers: SlotMap<InnerBufferHandle, BufferResource>,
    /// 存储所有的 Image 资源
    images: SlotMap<InnerImageHandle, ImageResource>,
    /// 存储所有的 ImageView 资源
    image_views: SlotMap<InnerImageViewHandle, ImageViewResource>,

    // 待销毁队列 (用于延迟销毁，例如在帧结束时)
    // (handle, frame_index)
    /// 待销毁的 Buffer 队列，存储 (Handle, 提交销毁时的帧索引)
    pending_destroy_buffers: Vec<(InnerBufferHandle, u64)>,
    /// 待销毁的 Image 队列
    pending_destroy_images: Vec<(InnerImageHandle, u64)>,
    /// 待销毁的 ImageView 队列
    pending_destroy_image_views: Vec<(InnerImageViewHandle, u64)>,

    /// 当前帧索引，用于判断资源是否可以安全销毁
    current_frame_index: u64,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager {
    /// 创建一个新的资源管理器
    pub fn new() -> Self {
        Self {
            buffers: SlotMap::with_key(),
            images: SlotMap::with_key(),
            image_views: SlotMap::with_key(),
            pending_destroy_buffers: Vec::new(),
            pending_destroy_images: Vec::new(),
            pending_destroy_image_views: Vec::new(),
            current_frame_index: 0,
        }
    }

    /// 设置当前帧索引
    ///
    /// 在每一帧开始时调用，用于更新资源管理器的内部时间戳。
    pub fn set_current_frame_index(&mut self, frame_index: u64) {
        self.current_frame_index = frame_index;
    }

    /// 清理已过期的资源
    ///
    /// 检查待销毁队列，销毁那些已经不再被 GPU 使用的资源（即提交销毁时的帧索引 <= completed_frame_index）。
    pub fn cleanup(&mut self, completed_frame_index: u64) {
        let _span = tracy_client::span!("ResourceManager::cleanup");

        // Clean up buffers
        let mut buffers_to_destroy = Vec::new();
        self.pending_destroy_buffers.retain(|(handle, frame_index)| {
            if *frame_index <= completed_frame_index {
                buffers_to_destroy.push(*handle);
                false
            } else {
                true
            }
        });
        for handle in buffers_to_destroy {
            if let Some(resource) = self.buffers.remove(handle) {
                self.destroy_buffer_resource(resource);
            }
        }

        // Clean up image views first (dependencies)
        let mut views_to_destroy = Vec::new();
        self.pending_destroy_image_views.retain(|(handle, frame_index)| {
            if *frame_index <= completed_frame_index {
                views_to_destroy.push(*handle);
                false
            } else {
                true
            }
        });
        for handle in views_to_destroy {
            if let Some(resource) = self.image_views.remove(handle) {
                self.destroy_image_view_resource(resource);
            }
        }

        // Clean up images
        let mut images_to_destroy = Vec::new();
        self.pending_destroy_images.retain(|(handle, frame_index)| {
            if *frame_index <= completed_frame_index {
                images_to_destroy.push(*handle);
                false
            } else {
                true
            }
        });
        for handle in images_to_destroy {
            if let Some(resource) = self.images.remove(handle) {
                self.destroy_image_resource(resource);
            }
        }
    }

    // --- Buffer API ---

    /// 创建一个 Buffer
    ///
    /// # 参数
    /// - `size`: Buffer 大小（字节）
    /// - `usage`: Vulkan Buffer 用途标志
    /// - `mapped`: 是否映射到主机内存（Host Visible）
    /// - `buffer_type`: Buffer 类型（用于内部标记）
    /// - `name`: 调试名称
    pub fn create_buffer(
        &mut self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        mapped: bool,
        buffer_type: BufferType,
        name: impl AsRef<str>,
    ) -> BufferHandle {
        let _span = tracy_client::span!("ResourceManager::create_buffer");
        let buffer_ci = vk::BufferCreateInfo::default().size(size).usage(usage);
        let alloc_ci = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            flags: if mapped {
                vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM
            } else {
                vk_mem::AllocationCreateFlags::empty()
            },
            ..Default::default()
        };

        let (buffer, mut alloc) =
            unsafe { Gfx::get().allocator().create_buffer_with_alignment(&buffer_ci, &alloc_ci, 8).unwrap() };

        let mut mapped_ptr = None;
        if mapped {
            unsafe {
                mapped_ptr = Some(Gfx::get().allocator().map_memory(&mut alloc).unwrap());
            }
        }

        let mut device_addr = None;
        if usage.contains(vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS) {
            let gfx_device = Gfx::get().gfx_device();
            unsafe {
                device_addr =
                    Some(gfx_device.get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(buffer)));
            }
        }

        Gfx::get().gfx_device().set_object_debug_name(buffer, format!("Buffer::{}", name.as_ref()));

        let resource = BufferResource {
            buffer,
            allocation: alloc,
            buffer_type,
            size,
            usage,
            mapped_ptr,
            device_addr,
            element_count: 0, // Default, can be updated later
            stride: 0,        // Default
            #[cfg(debug_assertions)]
            debug_name: name.as_ref().to_string(),
        };

        let inner = self.buffers.insert(resource);
        BufferHandle { inner }
    }

    /// 获取 Buffer 资源引用
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<&BufferResource> {
        self.buffers.get(handle.inner)
    }

    /// 获取 Buffer 资源可变引用
    pub fn get_buffer_mut(&mut self, handle: BufferHandle) -> Option<&mut BufferResource> {
        self.buffers.get_mut(handle.inner)
    }

    /// 销毁 Buffer（指定帧索引）
    ///
    /// 将 Buffer 加入待销毁队列，在 `current_frame_index` 对应的帧完成后销毁。
    pub fn destroy_buffer(&mut self, handle: BufferHandle, current_frame_index: u64) {
        self.pending_destroy_buffers.push((handle.inner, current_frame_index));
    }

    /// 自动销毁 Buffer
    ///
    /// 使用当前管理器的 `current_frame_index` 作为销毁时间点。
    pub fn destroy_buffer_auto(&mut self, handle: BufferHandle) {
        self.pending_destroy_buffers.push((handle.inner, self.current_frame_index));
    }

    /// 立即销毁 Buffer
    ///
    /// **警告**: 必须确保 GPU 不再使用该 Buffer，否则会导致未定义行为。
    pub fn destroy_buffer_immediate(&mut self, handle: BufferHandle) {
        let _span = tracy_client::span!("ResourceManager::destroy_buffer_immediate");
        if let Some(resource) = self.buffers.remove(handle.inner) {
            self.destroy_buffer_resource(resource);
        }
    }

    /// 内部方法：执行 Buffer 资源的实际销毁
    fn destroy_buffer_resource(&self, mut resource: BufferResource) {
        let allocator = Gfx::get().allocator();
        unsafe {
            if resource.mapped_ptr.is_some() {
                allocator.unmap_memory(&mut resource.allocation);
            }
            allocator.destroy_buffer(resource.buffer, &mut resource.allocation);
        }
    }

    /// 刷新 Buffer 内存（用于非 Coherent 内存）
    pub fn flush_buffer(&self, handle: BufferHandle, offset: vk::DeviceSize, size: vk::DeviceSize) {
        let _span = tracy_client::span!("ResourceManager::flush_buffer");
        if let Some(resource) = self.buffers.get(handle.inner) {
            let allocator = Gfx::get().allocator();
            allocator.flush_allocation(&resource.allocation, offset, size).unwrap();
        }
    }

    // --- Image API ---

    /// 创建一个 Image
    ///
    /// 同时会创建一个默认的 ImageView。
    pub fn create_image(
        &mut self,
        create_info: &vk::ImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        name: impl AsRef<str>,
    ) -> ImageHandle {
        let _span = tracy_client::span!("ResourceManager::create_image");
        let (image, alloc) = unsafe { Gfx::get().allocator().create_image(create_info, alloc_info).unwrap() };

        Gfx::get().gfx_device().set_object_debug_name(image, format!("Image::{}", name.as_ref()));

        let aspect_mask = Self::get_format_aspect_mask(create_info.format);
        let view_type = Self::image_type_to_view_type(create_info.image_type);

        // Create default view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(view_type)
            .format(create_info.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: create_info.mip_levels,
                base_array_layer: 0,
                layer_count: create_info.array_layers,
            });

        let view = unsafe { Gfx::get().gfx_device().create_image_view(&view_info, None).unwrap() };

        Gfx::get().gfx_device().set_object_debug_name(view, format!("ImageView::Default::{}", name.as_ref()));

        // Reserve slot for image to get handle
        let inner_image_handle = self.images.insert_with_key(|_k| ImageResource {
            image,
            source: ImageSource::Allocated(alloc),
            extent: create_info.extent,
            format: create_info.format,
            usage: create_info.usage,
            aspect_flags: aspect_mask,
            default_view: ImageViewHandle {
                inner: Default::default(),
            },
            #[cfg(debug_assertions)]
            debug_name: name.as_ref().to_string(),
        });

        let image_handle = ImageHandle {
            inner: inner_image_handle,
        };

        let view_resource = ImageViewResource {
            handle: view,
            image: image_handle,
            subresource_range: view_info.subresource_range,
            view_type: view_info.view_type,
            format: view_info.format,
        };

        let inner_view_handle = self.image_views.insert(view_resource);
        let view_handle = ImageViewHandle {
            inner: inner_view_handle,
        };

        // Update image with correct view handle
        if let Some(img) = self.images.get_mut(inner_image_handle) {
            img.default_view = view_handle;
        }

        image_handle
    }

    /// 创建 Image 并上传数据
    ///
    /// 这是一个便捷方法，内部会创建 Staging Buffer 并执行 Copy 命令。
    pub fn create_image_with_data(
        &mut self,
        create_info: &vk::ImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        data: &[u8],
        name: impl AsRef<str>,
    ) -> ImageHandle {
        let _span = tracy_client::span!("ResourceManager::create_image_with_data");
        // Ensure usage includes TRANSFER_DST
        let mut info = *create_info;
        info.usage |= vk::ImageUsageFlags::TRANSFER_DST;

        let handle = self.create_image(&info, alloc_info, &name);
        self.upload_image_data(handle, data);
        handle
    }

    /// 上传数据到 Image
    ///
    /// 使用 Staging Buffer 和 One-Time Command Buffer。
    pub fn upload_image_data(&mut self, image_handle: ImageHandle, data: &[u8]) {
        let _span = tracy_client::span!("ResourceManager::upload_image_data");
        let image_res = self.get_image(image_handle).expect("Invalid image handle");
        let width = image_res.extent.width;
        let height = image_res.extent.height;
        let depth = image_res.extent.depth;
        let vk_image = image_res.image;
        let aspect_mask = image_res.aspect_flags;

        // Create stage buffer
        let stage_buffer_handle = self.create_buffer(
            data.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            "image-upload-stage",
        );

        let stage_buffer_res = self.get_buffer_mut(stage_buffer_handle).unwrap();
        let vk_buffer = stage_buffer_res.buffer;

        // Copy data
        if let Some(ptr) = stage_buffer_res.mapped_ptr {
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
            }
        }

        // Execute command
        Gfx::get().one_time_exec(
            |cmd| {
                let image_barrier = GfxImageBarrier::new()
                    .image(vk_image)
                    .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                    .dst_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                    .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .image_aspect_flag(aspect_mask);
                cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));

                let buffer_image_copy = vk::BufferImageCopy2::default()
                    .buffer_offset(0)
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                    .image_extent(vk::Extent3D { width, height, depth })
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                cmd.cmd_copy_buffer_to_image(
                    &vk::CopyBufferToImageInfo2::default()
                        .src_buffer(vk_buffer)
                        .dst_image(vk_image)
                        .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .regions(std::slice::from_ref(&buffer_image_copy)),
                );

                let image_barrier = GfxImageBarrier::new()
                    .image(vk_image)
                    .src_mask(vk::PipelineStageFlags2::TRANSFER, vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ)
                    .layout_transfer(vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_aspect_flag(aspect_mask);
                cmd.image_memory_barrier(vk::DependencyFlags::empty(), std::slice::from_ref(&image_barrier));
            },
            "upload_image_data",
        );

        // Destroy stage buffer
        self.destroy_buffer_immediate(stage_buffer_handle);
    }

    /// 获取 Image 资源引用
    pub fn get_image(&self, handle: ImageHandle) -> Option<&ImageResource> {
        self.images.get(handle.inner)
    }

    /// 销毁 Image（指定帧索引）
    ///
    /// 同时会销毁默认的 ImageView。
    pub fn destroy_image(&mut self, handle: ImageHandle, current_frame_index: u64) {
        // Also destroy default view?
        // Yes, usually. But if we have other views, we should destroy them too?
        // The user should destroy other views manually.
        // But the default view is owned by the image conceptually.

        if let Some(img) = self.images.get(handle.inner) {
            self.destroy_image_view(img.default_view, current_frame_index);
        }

        self.pending_destroy_images.push((handle.inner, current_frame_index));
    }

    /// 自动销毁 Image
    pub fn destroy_image_auto(&mut self, handle: ImageHandle) {
        if let Some(img) = self.images.get(handle.inner) {
            self.destroy_image_view_auto(img.default_view);
        }
        self.pending_destroy_images.push((handle.inner, self.current_frame_index));
    }

    /// 自动销毁 ImageView
    pub fn destroy_image_view_auto(&mut self, handle: ImageViewHandle) {
        self.pending_destroy_image_views.push((handle.inner, self.current_frame_index));
    }

    /// 内部方法：执行 Image 资源的实际销毁
    fn destroy_image_resource(&self, resource: ImageResource) {
        match resource.source {
            ImageSource::Allocated(mut alloc) => unsafe {
                Gfx::get().allocator().destroy_image(resource.image, &mut alloc);
            },
            ImageSource::External => {
                // Do nothing for external images
            }
        }
    }

    // --- Image View API ---

    /// 创建一个 ImageView
    pub fn create_image_view(
        &mut self,
        image_handle: ImageHandle,
        view_info: &vk::ImageViewCreateInfo,
        name: impl AsRef<str>,
    ) -> ImageViewHandle {
        let _span = tracy_client::span!("ResourceManager::create_image_view");
        let image_resource = self.images.get(image_handle.inner).expect("Invalid image handle");

        // Override image in view_info with actual image handle
        let mut info = *view_info;
        info.image = image_resource.image;

        let view = unsafe { Gfx::get().gfx_device().create_image_view(&info, None).unwrap() };

        Gfx::get().gfx_device().set_object_debug_name(view, format!("ImageView::{}", name.as_ref()));

        let resource = ImageViewResource {
            handle: view,
            image: image_handle,
            subresource_range: info.subresource_range,
            view_type: info.view_type,
            format: info.format,
        };

        let inner = self.image_views.insert(resource);
        ImageViewHandle { inner }
    }

    /// 获取 ImageView 资源引用
    pub fn get_image_view(&self, handle: ImageViewHandle) -> Option<&ImageViewResource> {
        self.image_views.get(handle.inner)
    }

    /// 销毁 ImageView（指定帧索引）
    pub fn destroy_image_view(&mut self, handle: ImageViewHandle, current_frame_index: u64) {
        self.pending_destroy_image_views.push((handle.inner, current_frame_index));
    }

    /// 内部方法：执行 ImageView 资源的实际销毁
    fn destroy_image_view_resource(&self, resource: ImageViewResource) {
        unsafe {
            Gfx::get().gfx_device().destroy_image_view(resource.handle, None);
        }
    }

    /// 销毁所有资源
    ///
    /// 通常在程序退出时调用。
    pub fn destroy_all(&mut self) {
        let _span = tracy_client::span!("ResourceManager::destroy_all");
        // Destroy all image views
        let views: Vec<_> = self.image_views.drain().map(|(_, v)| v).collect();
        for view in views {
            self.destroy_image_view_resource(view);
        }

        // Destroy all images
        let images: Vec<_> = self.images.drain().map(|(_, i)| i).collect();
        for image in images {
            self.destroy_image_resource(image);
        }

        // Destroy all buffers
        let buffers: Vec<_> = self.buffers.drain().map(|(_, b)| b).collect();
        for buffer in buffers {
            self.destroy_buffer_resource(buffer);
        }

        // Clear pending queues
        self.pending_destroy_buffers.clear();
        self.pending_destroy_images.clear();
        self.pending_destroy_image_views.clear();
    }

    // --- Helper methods for creating specific types of buffers ---

    /// 创建顶点 Buffer
    pub fn create_vertex_buffer<L: GfxVertexLayout>(
        &mut self,
        vertex_cnt: usize,
        name: impl AsRef<str>,
    ) -> VertexBufferHandle<L> {
        let size = L::buffer_size(vertex_cnt) as vk::DeviceSize;
        let usage = vk::BufferUsageFlags::VERTEX_BUFFER
            | vk::BufferUsageFlags::TRANSFER_DST
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR;

        let handle = self.create_buffer(size, usage, false, BufferType::Vertex, name);

        // Update meta
        if let Some(resource) = self.buffers.get_mut(handle.inner) {
            resource.element_count = vertex_cnt as u32;
            // resource.stride = L::pos_stride(); // Optional, if we want to store stride
        }

        VertexBufferHandle {
            inner: handle.inner,
            _marker: PhantomData,
        }
    }

    /// 创建索引 Buffer
    pub fn create_index_buffer<T: GfxIndexType>(
        &mut self,
        index_cnt: usize,
        name: impl AsRef<str>,
    ) -> IndexBufferHandle {
        let size = (index_cnt * T::byte_size()) as vk::DeviceSize;
        let usage = vk::BufferUsageFlags::INDEX_BUFFER
            | vk::BufferUsageFlags::TRANSFER_DST
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
            | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR;

        let handle = self.create_buffer(size, usage, false, BufferType::Index, name);

        if let Some(resource) = self.buffers.get_mut(handle.inner) {
            resource.element_count = index_cnt as u32;
            resource.stride = T::byte_size() as u32;
        }

        IndexBufferHandle { inner: handle.inner }
    }

    /// 创建结构化 Buffer (Storage Buffer)
    pub fn create_structured_buffer<T: bytemuck::Pod>(
        &mut self,
        count: usize,
        usage: vk::BufferUsageFlags,
        mapped: bool,
        buffer_type: BufferType,
        name: impl AsRef<str>,
    ) -> StructuredBufferHandle<T> {
        let size = (count * size_of::<T>()) as vk::DeviceSize;

        // Ensure SHADER_DEVICE_ADDRESS is set if it's a storage/uniform buffer
        let final_usage = usage | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;

        let handle = self.create_buffer(size, final_usage, mapped, buffer_type, name);

        if let Some(resource) = self.buffers.get_mut(handle.inner) {
            resource.element_count = count as u32;
            resource.stride = size_of::<T>() as u32;
        }

        StructuredBufferHandle {
            inner: handle.inner,
            _marker: PhantomData,
        }
    }

    /// 创建 Uniform Buffer
    pub fn create_uniform_buffer<T: bytemuck::Pod>(
        &mut self,
        count: usize,
        name: impl AsRef<str>,
    ) -> StructuredBufferHandle<T> {
        self.create_structured_buffer(
            count,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            false,
            BufferType::Uniform,
            name,
        )
    }

    /// 创建 Staging Buffer (Host Visible, Transfer Src)
    pub fn create_stage_buffer<T: bytemuck::Pod>(
        &mut self,
        count: usize,
        name: impl AsRef<str>,
    ) -> StructuredBufferHandle<T> {
        self.create_structured_buffer(count, vk::BufferUsageFlags::TRANSFER_SRC, true, BufferType::Stage, name)
    }

    fn get_format_aspect_mask(format: vk::Format) -> vk::ImageAspectFlags {
        match format {
            vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::X8_D24_UNORM_PACK32 => {
                vk::ImageAspectFlags::DEPTH
            }
            vk::Format::D16_UNORM_S8_UINT | vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT => {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            }
            vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,
            _ => vk::ImageAspectFlags::COLOR,
        }
    }

    fn image_type_to_view_type(image_type: vk::ImageType) -> vk::ImageViewType {
        match image_type {
            vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
            vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
            vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
            _ => vk::ImageViewType::TYPE_2D,
        }
    }

    /// 创建外部 Image (例如 Swapchain Image)
    ///
    /// 外部 Image 不由 ResourceManager 管理内存生命周期，但会创建对应的 Handle 和 View。
    pub fn create_external_image(
        &mut self,
        image: vk::Image,
        create_info: &vk::ImageCreateInfo,
        name: impl AsRef<str>,
    ) -> ImageHandle {
        let _span = tracy_client::span!("ResourceManager::create_external_image");
        Gfx::get().gfx_device().set_object_debug_name(image, format!("Image::External::{}", name.as_ref()));

        let aspect_mask = Self::get_format_aspect_mask(create_info.format);
        let view_type = Self::image_type_to_view_type(create_info.image_type);

        // Create default view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(view_type)
            .format(create_info.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: create_info.mip_levels,
                base_array_layer: 0,
                layer_count: create_info.array_layers,
            });

        let view = unsafe { Gfx::get().gfx_device().create_image_view(&view_info, None).unwrap() };

        Gfx::get().gfx_device().set_object_debug_name(view, format!("ImageView::External::Default::{}", name.as_ref()));

        let inner_image_handle = self.images.insert_with_key(|_| ImageResource {
            image,
            source: ImageSource::External,
            extent: create_info.extent,
            format: create_info.format,
            usage: create_info.usage,
            aspect_flags: aspect_mask,
            default_view: ImageViewHandle {
                inner: Default::default(),
            },
            #[cfg(debug_assertions)]
            debug_name: name.as_ref().to_string(),
        });

        let image_handle = ImageHandle {
            inner: inner_image_handle,
        };

        let view_resource = ImageViewResource {
            handle: view,
            image: image_handle,
            subresource_range: view_info.subresource_range,
            view_type: view_info.view_type,
            format: view_info.format,
        };

        let inner_view_handle = self.image_views.insert(view_resource);
        let view_handle = ImageViewHandle {
            inner: inner_view_handle,
        };

        if let Some(img) = self.images.get_mut(inner_image_handle) {
            img.default_view = view_handle;
        }

        image_handle
    }

    pub fn get_vertex_buffer<L: GfxVertexLayout>(&self, handle: VertexBufferHandle<L>) -> Option<&BufferResource> {
        self.buffers.get(handle.inner)
    }

    pub fn get_index_buffer(&self, handle: IndexBufferHandle) -> Option<&BufferResource> {
        self.buffers.get(handle.inner)
    }
}
