# truvis-rhi

Truvis 的 RHI (Render Hardware Interface) 层，提供 Vulkan 的现代化封装和抽象。

## 🎯 设计目标

- **现代 Vulkan API 支持**: 基于 Vulkan 1.3，支持动态渲染、光线追踪、加速结构
- **内存安全**: Rust 类型系统确保对象生命周期安全
- **高性能**: 零成本抽象，直接映射 Vulkan API
- **职责分离**: 专注于 Vulkan API 抽象，不包含高级资源管理策略

## 📋 架构原则

### 分层设计
```
应用层 (OuterApp) → 渲染器层 (truvis-render) → RHI 抽象层 (truvis-rhi) → Vulkan API
```

**RHI 层职责**:
- 设备和队列管理
- 基础资源创建 (缓冲区、图像)
- 命令记录和同步
- 内存分配 (基于 vk-mem)

**不包含的功能** (由上层负责):
- 资源生命周期管理和缓存
- 纹理加载和格式转换
- 渲染图和帧同步策略

## 📁 代码结构

### 核心模块 (`src/core/`)
- **`device.rs`**: Vulkan 逻辑设备，支持光线追踪和动态渲染
- **`instance.rs`**: Vulkan 实例管理，验证层集成
- **`physical_device.rs`**: 物理设备选择和能力检测
- **`allocator.rs`**: VMA 内存分配器封装
- **`command_buffer.rs`**: 命令缓冲区记录
- **`command_pool.rs`** / **`command_queue.rs`**: 命令池和队列管理
- **`acceleration.rs`**: 光线追踪加速结构 (BLAS/TLAS)
- **`graphics_pipeline.rs`**: 图形管线状态对象
- **`descriptor.rs`**: 描述符管理和 Bindless 支持
- **`synchronize.rs`**: 同步原语 (屏障、围栏、信号量)

### 资源管理 (`src/resources/`)
提供句柄式资源管理，作为 RHI 和上层的桥梁：
- **`managed_buffer.rs`** / **`managed_image.rs`**: VMA 集成的智能资源
- **`resource_manager.rs`**: 句柄式资源管理器
- **`resource_handles.rs`**: 类型安全的资源句柄
- **`resource_creator.rs`**: 统一的资源创建接口

### 其他模块
- **`src/basic/`**: 基础类型定义
- **`src/shader_cursor/`**: 着色器模块加载

## 🚀 核心 API

### Rhi 主结构体
```rust
pub struct Rhi {
    pub vk_pf: Rc<ash::Entry>,                    // Vulkan 动态库入口
    pub device: Rc<RhiDevice>,                    // 逻辑设备
    pub allocator: Rc<RhiAllocator>,              // VMA 内存分配器
    
    // 专用队列
    pub graphics_queue: Rc<RhiQueue>,
    pub compute_queue: Rc<RhiQueue>,
    pub transfer_queue: Rc<RhiQueue>,
    
    pub temp_graphics_command_pool: Rc<RhiCommandPool>, // 临时命令池
}
```

### 基础用法

**1. 初始化 RHI**
```rust
let rhi = Rhi::new("MyApp".to_string(), vec![]);
```

**2. 创建缓冲区**
```rust
// 设备本地缓冲区 (GPU 高速内存)
let vertex_buffer = RhiBuffer::new_device_buffer(
    &rhi, size, vk::BufferUsageFlags::VERTEX_BUFFER, "vertex-buffer"
);

// 暂存缓冲区 (CPU 可访问)
let stage_buffer = RhiBuffer::new_stage_buffer(&rhi, size, "stage-buffer");
```

**3. 命令记录**
```rust
let cmd = rhi.temp_graphics_command_pool.alloc_command_buffer("render-pass");
cmd.begin_command_buffer();

// 动态渲染 (Vulkan 1.3)
cmd.cmd_begin_rendering(&rendering_info);
cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, pipeline);
cmd.cmd_draw(vertex_count, 1, 0, 0);
cmd.cmd_end_rendering();

cmd.end_command_buffer();
graphics_queue.submit(&[cmd.handle()], &[], &[], fence);
```

## 🔧 关键特性

### 现代 Vulkan 支持
- **动态渲染**: 无需 RenderPass，直接渲染到附件
- **光线追踪**: BLAS/TLAS 加速结构，RT 管线支持
- **Bindless**: 运行时描述符数组，减少绑定切换
- **设备地址**: 缓冲区设备地址，GPU 指针访问

### 内存管理
- **VMA 集成**: 自动 GPU 内存分配和对齐
- **生命周期安全**: Rust RAII 确保资源正确释放
- **句柄系统**: 避免悬空指针，支持延迟清理

### 队列分离
- **专用队列**: 图形、计算、传输队列独立操作
- **并行性**: 减少队列争用，提升性能
- **异步传输**: 支持后台数据传输

## 📦 资源管理

### 句柄式资源管理
RHI 层提供句柄式资源管理，作为底层 Vulkan 和上层渲染器的桥梁：

```rust
/// 句柄式资源管理器
pub struct RhiResourceManager {
    images: HashMap<ImageHandle, ManagedImage2D>,
    buffers: HashMap<BufferHandle, ManagedBuffer>,
    image_views: HashMap<ImageViewHandle, ManagedImage2DView>,
    // ID 生成器
    next_image_id: u64,
    next_buffer_id: u64,
    next_view_id: u64,
}
```

### 智能资源类型
- **`ManagedImage2D`**: VMA 集成的图像，自动生命周期管理
- **`ManagedBuffer`**: VMA 集成的缓冲区，支持映射和传输
- **`ManagedImage2DView`**: 图像视图管理，自动清理

### 资源使用模式
```rust
// 注册资源，获得句柄
let image_handle = manager.register_image(managed_image);

// 通过句柄访问资源
if let Some(image) = manager.get_image(image_handle) {
    let vk_image = image.handle();
    // 使用 Vulkan 图像句柄
}

// 自动清理
manager.cleanup_unused_resources();
```

### 设计原则
- **RHI 职责**: 提供基础资源创建和句柄管理
- **上层职责**: 实现缓存、池化等高级策略
- **边界清晰**: 避免在 RHI 层实现业务逻辑

## ⚠️ 使用注意事项

### 架构边界
**✅ RHI 应该提供**:
- 基础 Vulkan 对象的类型安全封装
- 设备、队列、内存分配器的抽象  
- 资源创建的工厂方法
- 句柄式资源管理系统

**❌ RHI 不应该包含**:
- 资源缓存和去重逻辑
- 渲染图和依赖关系管理
- 场景图和GPU数据同步
- 资源池化和批量优化

### 初始化顺序
1. Entry → Instance → PhysicalDevice → Device → Allocator
2. VMA 分配器必须在 Device 创建后初始化
3. 队列必须在 Device 创建后获取

### 内存对齐
- UBO/SSBO 数据必须满足 `std140`/`std430` 对齐
- 加速结构缓冲区需要 256 字节对齐
- VMA 自动处理缓冲区内存对齐

### 生命周期管理
- 确保 `Rhi` 在所有资源之前销毁
- 命令缓冲区必须在命令池之前销毁
- 描述符集必须在描述符池之前销毁

### 常见陷阱
```rust
// ❌ 错误：viewport 设置
let viewport = vk::Viewport { height: extent.height as f32, .. };

// ✅ 正确：Y轴翻转 (height < 0)
let viewport = vk::Viewport { 
    y: extent.height as f32,
    height: -(extent.height as f32),
    ..
};
```

## 🔄 与上层集成

### 推荐的资源管理模式
```rust
// truvis-render 中的正确使用方式
pub struct RenderResources {
    pub resource_manager: RhiResourceManager, // RHI 提供的句柄系统
    pub bindless_mgr: BindlessManager,        // 高级 bindless 管理
    pub texture_cache: TextureCache,          // 纹理缓存策略 (上层实现)
}

impl RenderResources {
    pub fn create_texture_from_file(&mut self, rhi: &Rhi, path: &Path) -> ImageHandle {
        // 1. 检查缓存 (上层策略)
        if let Some(handle) = self.texture_cache.get(path) {
            return handle;
        }
        
        // 2. 通过 RHI 创建基础资源
        let managed_image = ManagedImage2D::from_file(rhi, path);
        
        // 3. 注册到 RHI 句柄系统
        let handle = self.resource_manager.register_image(managed_image);
        
        // 4. 缓存结果 (上层策略)
        self.texture_cache.insert(path, handle);
        handle
    }
}
```

### 最佳实践
- **分离关注点**: RHI 专注 Vulkan 抽象，上层处理业务逻辑
- **使用句柄**: 通过句柄访问资源，避免悬空指针
- **延迟清理**: 利用句柄系统实现资源的延迟清理
- **缓存在上层**: 所有缓存和优化策略在 `truvis-render` 实现
