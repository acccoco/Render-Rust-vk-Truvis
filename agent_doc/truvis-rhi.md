# truvis-rhi

## 概述
Truvis 的 RHI (Render Hardware Interface) 层，提供 Vulkan 的封装和抽象。这是整个渲染引擎的底层图形 API 抽象层。

## 架构组织

### 核心模块 (`src/core/`)
具体文件结构：
- **`device.rs`**: Vulkan 逻辑设备的创建和管理
- **`instance.rs`**: Vulkan 实例和扩展管理
- **`physical_device.rs`**: 物理设备选择和属性查询
- **`command_buffer.rs`**: 命令缓冲区记录和执行
- **`command_pool.rs`**: 命令池管理
- **`command_queue.rs`**: 队列管理（图形、计算、传输）
- **`graphics_pipeline.rs`**: 图形管线状态对象
- **`descriptor.rs`**: 描述符集和布局
- **`descriptor_pool.rs`**: 描述符池管理
- **`synchronize.rs`**: 同步原语（围栏、信号量、屏障）
- **`allocator.rs`**: VMA 内存分配器封装
- **`acceleration.rs`**: 光线追踪加速结构
- **`sampler.rs`**: 采样器对象
- **`shader.rs`**: 着色器模块
- **`query_pool.rs`**: 查询池管理
- **`rendering_info.rs`**: 动态渲染信息
- **`debug_utils.rs`**: 调试工具和验证层

### 资源管理 (`src/resources/`)
具体文件结构：
- **`managed_buffer.rs`**: 托管缓冲区（集成 VMA）
- **`managed_image.rs`**: 托管图像资源
- **`managed_image_view.rs`**: 图像视图管理
- **`resource_creator.rs`**: 资源创建工厂
- **`resource_manager.rs`**: 资源生命周期管理
- **`resource_handles.rs`**: 资源句柄类型定义
- **`creator.rs`**: 资源创建辅助函数

### 基础设施 (`src/basic/`)
- 基础类型定义和常用工具
- Vulkan 对象的 Rust 封装

### 着色器支持 (`src/shader_cursor/`)
- 着色器模块的加载和管理
- 着色器反射信息处理

## 关键特性
- **类型安全**: 使用 Rust 类型系统确保 Vulkan API 调用的安全性
- **内存管理**: 集成 VMA 提供高效的 GPU 内存分配
- **资源生命周期**: 自动管理 Vulkan 对象的生命周期
- **调试支持**: 内置验证层和调试工具支持

## 依赖关系
- `ash`: Vulkan API 绑定
- `vk-mem`: Vulkan Memory Allocator 绑定
- `winit`: 窗口和表面创建
- `shader-layout-*`: 着色器布局宏和 trait

## 使用模式

### RHI 初始化（来自 `src/rhi.rs`）
```rust
pub struct Rhi {
    pub vk_pf: Rc<ash::Entry>,
    instance: Rc<RhiInstance>,
    physical_device: Rc<RhiPhysicalDevice>,
    pub device: Rc<RhiDevice>,
    pub allocator: Rc<RhiAllocator>,
    pub temp_graphics_command_pool: Rc<RhiCommandPool>,
    pub graphics_queue: Rc<RhiQueue>,
    pub compute_queue: Rc<RhiQueue>,
    pub transfer_queue: Rc<RhiQueue>,
}

impl Rhi {
    pub fn new(app_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        // 实际实现见 crates/truvis-rhi/src/rhi.rs
    }
}
```

### 资源创建模式（基于 `src/resources/` 模块）
```rust
// 托管缓冲区创建
let buffer = RhiManagedBuffer::new(&rhi.device, &rhi.allocator, size, usage)?;

// 托管图像创建
let image = RhiManagedImage::new(&rhi.device, &rhi.allocator, image_info)?;
```

## 开发注意事项
- 所有 Vulkan 对象都有对应的 Rust 包装器
- 内存管理通过 VMA 自动化处理
- 支持调试模式下的额外验证
- 与上层渲染模块通过统一接口交互
