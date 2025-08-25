# RHI 架构重构方案

## 目录
- [问题分析](#问题分析)
- [设计目标](#设计目标)
- [核心设计理念](#核心设计理念)
- [重构方案](#重构方案)
- [实现细节](#实现细节)
- [迁移策略](#迁移策略)
- [参考案例](#参考案例)

---

## 问题分析

### 当前架构的问题

1. **循环引用风险**
   - `RhiBuffer` 和 `RhiImage2D` 直接持有 `Rc<RhiDevice>` 强引用
   - 可能导致设备无法正确销毁，造成内存泄漏

2. **资源管理分散**
   - 每个资源独立管理自己的生命周期
   - 难以统一监控和优化资源使用情况
   - 缺乏统一的资源清理机制

3. **API 不一致**
   - 不同资源的创建和管理方式不统一
   - 缺乏类型安全的资源引用机制

4. **外部资源处理复杂**
   - Swapchain 自动创建的 Image 难以纳入统一管理

---

## 设计目标

1. **消除循环引用**：确保资源能够正确释放
2. **统一资源管理**：集中管理所有 GPU 资源的生命周期
3. **类型安全**：通过句柄系统避免悬空指针
4. **性能优化**：支持资源池化和批量操作
5. **向后兼容**：渐进式迁移，保持现有 API 可用

---

## 核心设计理念

### 句柄 + 资源管理器模式

采用现代游戏引擎常用的设计模式：
- **句柄系统**：轻量级的资源标识符，类型安全
- **集中管理**：单一的资源管理器负责所有资源
- **生命周期控制**：明确的资源创建和销毁时机
- **关系追踪**：自动管理资源之间的依赖关系

### 参考案例

- **Bevy Engine**：ECS + Asset 系统，逻辑资源与 GPU 资源分离
- **wgpu**：直接所有权 + 生命周期管理，简单直观的 API
- **现代游戏引擎**：普遍采用句柄 + 管理器的模式

---

## 重构方案

### 1. 句柄定义

```rust
use std::marker::PhantomData;

/// 通用句柄类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetHandle<T>(u64, PhantomData<T>);

/// 具体的句柄类型
pub type ImageHandle = AssetHandle<ManagedImage>;
pub type BufferHandle = AssetHandle<ManagedBuffer>;
pub type ImageViewHandle = AssetHandle<ManagedImageView>;

/// 外部资源句柄（用于 Swapchain 等）
pub type ExternalImageHandle = AssetHandle<ExternalImage>;
pub type ExternalImageViewHandle = AssetHandle<ExternalImageView>;
```

### 2. 资源结构重构

```rust
/// 管理的 Image 资源（去除设备引用）
pub struct ManagedImage {
    handle: vk::Image,
    allocation: vk_mem::Allocation,
    width: u32,
    height: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    debug_name: String,
    // 注意：不再持有 device 或 allocator 的强引用
}

/// 管理的 Buffer 资源
pub struct ManagedBuffer {
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    debug_name: String,
}

/// 管理的 ImageView 资源
pub struct ManagedImageView {
    handle: vk::ImageView,
    uuid: Image2DViewUUID,
    info: RhiImageViewCreateInfo,
    debug_name: String,
    image_handle: ImageHandle, // 关联的图像句柄
}

/// 外部 Image 资源（如 Swapchain）
pub struct ExternalImage {
    handle: vk::Image,
    width: u32,
    height: u32,
    format: vk::Format,
    debug_name: String,
    // 注意：没有 allocation，不需要我们释放
}
```

### 3. 资源管理器

```rust
use std::collections::HashMap;
use std::cell::RefCell;

/// 核心资源管理器
pub struct RhiResourceManager {
    // 自管理资源
    images: HashMap<ImageHandle, ManagedImage>,
    buffers: HashMap<BufferHandle, ManagedBuffer>,
    image_views: HashMap<ImageViewHandle, ManagedImageView>,
    
    // 外部资源
    external_images: HashMap<ExternalImageHandle, ExternalImage>,
    external_image_views: HashMap<ExternalImageViewHandle, ExternalImageView>,
    
    // 关系映射
    image_to_views: HashMap<ImageHandle, Vec<ImageViewHandle>>,
    external_image_to_views: HashMap<ExternalImageHandle, Vec<ExternalImageViewHandle>>,
    
    // ID 生成器
    next_image_id: u64,
    next_buffer_id: u64,
    next_view_id: u64,
    next_external_image_id: u64,
    next_external_view_id: u64,
}

impl RhiResourceManager {
    /// 创建 Buffer
    pub fn create_buffer(
        &mut self,
        device: &RhiDevice,
        allocator: &RhiAllocator,
        buffer_info: &RhiBufferCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> BufferHandle {
        let (buffer, allocation) = unsafe {
            allocator.create_buffer(buffer_info.info(), alloc_info).unwrap()
        };
        
        let handle = BufferHandle(AssetHandle(self.next_buffer_id, PhantomData));
        self.next_buffer_id += 1;
        
        let managed_buffer = ManagedBuffer {
            handle: buffer,
            allocation,
            size: buffer_info.size(),
            usage: buffer_info.info().usage,
            debug_name: debug_name.to_string(),
        };
        
        device.debug_utils().set_debug_name_raw(buffer, debug_name);
        self.buffers.insert(handle, managed_buffer);
        
        handle
    }
    
    /// 创建 Image
    pub fn create_image(
        &mut self,
        device: &RhiDevice,
        allocator: &RhiAllocator,
        image_info: &RhiImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> ImageHandle {
        let (image, allocation) = unsafe {
            allocator.create_image(image_info.creat_info(), alloc_info).unwrap()
        };
        
        let handle = ImageHandle(AssetHandle(self.next_image_id, PhantomData));
        self.next_image_id += 1;
        
        let managed_image = ManagedImage {
            handle: image,
            allocation,
            width: image_info.extent().width,
            height: image_info.extent().height,
            format: image_info.format(),
            usage: image_info.info().usage,
            debug_name: debug_name.to_string(),
        };
        
        device.debug_utils().set_debug_name_raw(image, debug_name);
        self.images.insert(handle, managed_image);
        self.image_to_views.insert(handle, Vec::new());
        
        handle
    }
    
    /// 注册外部 Image（如 Swapchain）
    pub fn register_external_image(
        &mut self,
        image: vk::Image,
        width: u32,
        height: u32,
        format: vk::Format,
        debug_name: &str,
    ) -> ExternalImageHandle {
        let handle = ExternalImageHandle(AssetHandle(self.next_external_image_id, PhantomData));
        self.next_external_image_id += 1;
        
        let external_image = ExternalImage {
            handle: image,
            width,
            height,
            format,
            debug_name: debug_name.to_string(),
        };
        
        self.external_images.insert(handle, external_image);
        self.external_image_to_views.insert(handle, Vec::new());
        
        handle
    }
    
    /// 销毁资源时自动处理依赖关系
    pub fn destroy_image(
        &mut self,
        device: &RhiDevice,
        allocator: &RhiAllocator,
        handle: ImageHandle,
    ) -> bool {
        // 先销毁所有关联的 ImageView
        if let Some(view_handles) = self.image_to_views.remove(&handle) {
            for view_handle in view_handles {
                self.destroy_image_view(device, view_handle);
            }
        }
        
        // 销毁 Image
        if let Some(mut image) = self.images.remove(&handle) {
            unsafe {
                allocator.destroy_image(image.handle, &mut image.allocation);
            }
            true
        } else {
            false
        }
    }
    
    // 资源访问方法
    pub fn get_image(&self, handle: ImageHandle) -> Option<&ManagedImage> {
        self.images.get(&handle)
    }
    
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<&ManagedBuffer> {
        self.buffers.get(&handle)
    }
    
    // ... 其他方法
}
```

### 4. 重构后的 Rhi

```rust
/// 重构后的 Rhi 结构
pub struct Rhi {
    // 保持原有的核心组件
    pub instance: Rc<RhiInstance>,
    pub device: Rc<RhiDevice>,
    pub allocator: Rc<RhiAllocator>,
    pub graphics_queue: RhiQueue,
    pub temp_graphics_command_pool: Rc<RhiCommandPool>,
    
    // 新增：统一的资源管理器
    resource_manager: RefCell<RhiResourceManager>,
}

impl Rhi {
    /// 新的句柄式 API
    pub fn create_image_2d(
        &self,
        image_info: &RhiImageCreateInfo,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> ImageHandle {
        self.resource_manager.borrow_mut().create_image(
            &self.device,
            &self.allocator,
            image_info,
            alloc_info,
            debug_name,
        )
    }
    
    /// 便利方法：创建纹理
    pub fn create_texture_from_rgba8(
        &self,
        width: u32,
        height: u32,
        data: &[u8],
        debug_name: &str,
    ) -> ImageHandle {
        let image_info = RhiImageCreateInfo::new_image_2d_info(
            vk::Extent2D { width, height },
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        );
        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        
        let handle = self.create_image_2d(&image_info, &alloc_info, debug_name);
        self.upload_image_data(handle, data);
        handle
    }
    
    /// Swapchain 专用 API
    pub fn register_swapchain_images(
        &self,
        images: &[vk::Image],
        extent: vk::Extent2D,
        format: vk::Format,
        name_prefix: &str,
    ) -> Vec<ExternalImageHandle> {
        let mut handles = Vec::new();
        let mut manager = self.resource_manager.borrow_mut();
        
        for (i, &image) in images.iter().enumerate() {
            let debug_name = format!("{}-{}", name_prefix, i);
            let handle = manager.register_external_image(
                image,
                extent.width,
                extent.height,
                format,
                &debug_name,
            );
            handles.push(handle);
        }
        
        handles
    }
    
    /// 资源访问方法
    pub fn with_image<F, R>(&self, handle: ImageHandle, f: F) -> Option<R>
    where
        F: FnOnce(&ManagedImage) -> R,
    {
        let manager = self.resource_manager.borrow();
        manager.get_image(handle).map(f)
    }
    
    /// 获取 Vulkan 句柄
    pub fn get_image_handle(&self, handle: ImageHandle) -> Option<vk::Image> {
        self.with_image(handle, |img| img.handle)
    }
    
    // 保持现有的直接创建 API（向后兼容）
    pub fn create_image_2d_direct(
        &self,
        image_info: Rc<RhiImageCreateInfo>,
        alloc_info: &vk_mem::AllocationCreateInfo,
        debug_name: &str,
    ) -> RhiImage2D {
        // 现有的实现保持不变
        RhiImage2D::new(self, image_info, alloc_info, debug_name)
    }
}
```

---

## 实现细节

### 1. Swapchain 集成

```rust
pub struct RenderSwapchain {
    // 原有字段
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    extent: vk::Extent2D,
    format: vk::SurfaceFormatKHR,
    current_image_index: usize,
    
    // 新增：资源管理句柄
    managed_image_handles: Vec<ExternalImageHandle>,
    managed_view_handles: Vec<ExternalImageViewHandle>,
}

impl RenderSwapchain {
    pub fn new(rhi: &Rhi, /* other params */) -> Self {
        // 创建 swapchain...
        let images = unsafe { 
            rhi.swapchain_loader.get_swapchain_images(swapchain).unwrap() 
        };
        
        // 注册到资源管理器
        let managed_image_handles = rhi.register_swapchain_images(
            &images,
            extent,
            surface_format.format,
            "swapchain-image",
        );
        
        let managed_view_handles = rhi.create_swapchain_image_views(
            &managed_image_handles,
            surface_format.format,
            "swapchain-image-view",
        );
        
        Self {
            swapchain,
            images,
            extent,
            format: surface_format,
            current_image_index: 0,
            managed_image_handles,
            managed_view_handles,
        }
    }
    
    pub fn current_image_handle(&self) -> ExternalImageHandle {
        self.managed_image_handles[self.current_image_index]
    }
}
```

### 2. 兼容性包装

```rust
/// 为现有代码提供兼容性包装
pub struct RhiImage2DRef<'a> {
    rhi: &'a Rhi,
    handle: ImageHandle,
}

impl<'a> RhiImage2DRef<'a> {
    pub fn width(&self) -> Option<u32> {
        self.rhi.with_image(self.handle, |img| img.width)
    }
    
    pub fn height(&self) -> Option<u32> {
        self.rhi.with_image(self.handle, |img| img.height)
    }
    
    pub fn handle(&self) -> Option<vk::Image> {
        self.rhi.get_image_handle(self.handle)
    }
}

impl RhiImage2D {
    /// 从句柄创建引用包装器
    pub fn from_handle(rhi: &Rhi, handle: ImageHandle) -> Option<RhiImage2DRef> {
        rhi.with_image(handle, |_| RhiImage2DRef { rhi, handle })
    }
}
```

### 3. 错误处理

```rust
/// 资源操作错误类型
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Invalid handle: {0:?}")]
    InvalidHandle(String),
    
    #[error("Resource already destroyed")]
    ResourceDestroyed,
    
    #[error("Vulkan error: {0}")]
    VulkanError(#[from] vk::Result),
    
    #[error("Allocation error: {0}")]
    AllocationError(#[from] vk_mem::Error),
}

/// 返回 Result 而不是 Option
impl Rhi {
    pub fn get_image_handle_checked(&self, handle: ImageHandle) -> Result<vk::Image, ResourceError> {
        self.with_image(handle, |img| img.handle)
            .ok_or_else(|| ResourceError::InvalidHandle(format!("{:?}", handle)))
    }
}
```

---

## 迁移策略

### 阶段 1：添加新 API（保持兼容）

1. 在 `Rhi` 中添加 `RhiResourceManager`
2. 实现新的句柄式 API
3. 保持所有现有 API 不变
4. 添加单元测试验证新 API

```rust
// 同时支持两种 API
let image_old = rhi.create_image_2d_direct(info, alloc_info, "test");  // 旧 API
let image_new = rhi.create_image_2d(&info, alloc_info, "test");        // 新 API
```

### 阶段 2：逐步迁移现有代码

1. 从简单的创建逻辑开始迁移
2. 更新 Swapchain 相关代码
3. 迁移渲染管线中的资源使用
4. 更新文档和示例

### 阶段 3：标记旧 API 为废弃

```rust
#[deprecated(since = "0.3.0", note = "Use create_image_2d instead")]
pub fn create_image_2d_direct(&self, ...) -> RhiImage2D {
    // 旧实现
}
```

### 阶段 4：移除旧 API

1. 完全移除废弃的 API
2. 简化内部实现
3. 更新依赖代码

---

## 性能考虑

### 优势

1. **减少引用计数开销**：不再使用 `Rc<RhiDevice>`
2. **批量操作**：可以实现批量资源创建和销毁
3. **内存局部性**：资源集中存储，提高缓存命中率
4. **资源池化**：容易实现资源重用机制

### 潜在开销

1. **额外的 HashMap 查找**：通过句柄访问资源需要查表
2. **RefCell 运行时借用检查**：可能有轻微的运行时开销

### 优化策略

```rust
/// 缓存频繁访问的资源句柄
pub struct CachedResourceAccess {
    image_cache: HashMap<ImageHandle, vk::Image>,
    buffer_cache: HashMap<BufferHandle, vk::Buffer>,
}

impl Rhi {
    /// 批量获取 Vulkan 句柄，减少锁争用
    pub fn get_vulkan_handles(&self, handles: &[ImageHandle]) -> Vec<Option<vk::Image>> {
        let manager = self.resource_manager.borrow();
        handles.iter()
            .map(|&handle| manager.get_image(handle).map(|img| img.handle))
            .collect()
    }
}
```

---

## 测试策略

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_lifecycle() {
        let rhi = create_test_rhi();
        
        // 创建资源
        let image = rhi.create_texture_2d(512, 512, vk::Format::R8G8B8A8_UNORM, "test");
        
        // 验证资源存在
        assert!(rhi.with_image(image, |img| img.width) == Some(512));
        
        // 创建 ImageView
        let view = rhi.create_image_view(image, /* params */).unwrap();
        
        // 销毁 Image 应该自动销毁 View
        assert!(rhi.destroy_image(image));
        
        // 验证资源已销毁
        assert!(rhi.with_image(image, |_| ()).is_none());
    }
    
    #[test]
    fn test_swapchain_integration() {
        let rhi = create_test_rhi();
        let mock_images = vec![vk::Image::null(); 3];
        
        let handles = rhi.register_swapchain_images(
            &mock_images,
            vk::Extent2D { width: 800, height: 600 },
            vk::Format::B8G8R8A8_UNORM,
            "test-swapchain",
        );
        
        assert_eq!(handles.len(), 3);
        
        // 验证可以访问
        for handle in &handles {
            assert!(rhi.with_external_image(*handle, |img| img.width) == Some(800));
        }
    }
}
```

### 集成测试

1. **内存泄漏检测**：使用 valgrind 或 Address Sanitizer
2. **性能基准测试**：对比新旧 API 的性能差异
3. **并发安全测试**：验证多线程环境下的安全性

---

## 总结

这个重构方案通过引入句柄 + 资源管理器模式，解决了当前架构中的循环引用问题，同时提供了更好的资源管理能力和类型安全性。

### 主要收益

1. **解决循环引用**：彻底消除内存泄漏风险
2. **统一资源管理**：集中管理所有 GPU 资源
3. **提升类型安全**：句柄系统避免悬空指针
4. **改善性能**：支持批量操作和资源优化
5. **保持兼容性**：渐进式迁移，降低迁移成本

### 实施建议

1. **从简单开始**：先实现 Buffer 的重构，积累经验
2. **充分测试**：每个阶段都要有完整的测试覆盖
3. **文档先行**：及时更新文档和示例代码
4. **性能监控**：密切关注重构对性能的影响

这个方案参考了 Bevy、wgpu 等现代图形库的最佳实践，既保证了技术先进性，又考虑了实际的迁移成本，是一个平衡且可行的重构方案。
