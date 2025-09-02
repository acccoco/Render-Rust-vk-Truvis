# RHI 命名重构提案

## 当前问题分析

### 1. `Rhi` 结构体职责过重
当前 `Rhi` 承担了太多职责：
- Vulkan 上下文管理  
- 资源管理
- 内存分配
- 命令执行

### 2. `Rhi` 前缀过度使用
在 `truvis-rhi` crate 内部，所有类型都使用 `Rhi` 前缀是冗余的。

### 3. `VulkanContext` 名称模糊
`VulkanContext` 实际管理的是 Vulkan 核心对象。

## 重构建议

### 阶段 1: 重命名核心类型（保持向后兼容）

```rust
// 原名 -> 新名 (通过 type alias 保持兼容)
pub type Rhi = RenderDevice;
pub type VulkanContext = VulkanCore;

// 在 crate 内部移除 Rhi 前缀
pub struct Device { /* 原 RhiDevice */ }
pub struct DeviceFunctions { /* 原 RhiDeviceFunctions */ }
pub struct CommandBuffer { /* 原 RhiCommandBuffer */ }
pub struct Queue { /* 原 RhiQueue */ }
// ... 其他类型

// 在 crate 根部重新导出（保持 API 兼容）
pub use device::Device as RhiDevice;
pub use commands::CommandBuffer as RhiCommandBuffer;
// ...
```

### 阶段 2: 重构主要结构体

**关键问题**: 如果将 `Rhi` 改名为 `RenderDevice`，会与 `RhiDevice` 产生混淆。

**层次结构分析**:
- `RhiDeviceFunctions`: Vulkan 设备函数指针集合（低层 API 封装）
- `RhiDevice`: Vulkan 逻辑设备封装（中层抽象）  
- `Rhi`: RHI 主入口点，管理整个渲染上下文（高层抽象）

**更好的命名方案**:

```rust
/// RHI 主入口点 - 渲染上下文管理器
pub struct RenderContext {  // 而不是 RenderDevice
    /// Vulkan 核心对象和上下文
    pub(crate) core: VulkanCore,
    /// 内存分配器
    pub(crate) allocator: Rc<MemoryAllocator>,
    /// 资源管理器
    pub(crate) resources: RefCell<ResourceManager>,
    /// 临时命令管理
    pub(crate) temp_commands: CommandManager,
}

// 或者考虑其他选项：
// pub struct RenderBackend { ... }    // 强调后端抽象
// pub struct GraphicsAPI { ... }      // 强调图形 API 抽象
// pub struct RenderInterface { ... }  // 强调接口抽象

**层次化命名策略**:

```rust
// === 低层：Vulkan 对象直接封装 ===
pub struct DeviceFunctions {    // 原 RhiDeviceFunctions  
    // Vulkan 函数指针集合
}

pub struct Device {             // 原 RhiDevice
    // Vulkan 逻辑设备封装
    pub(crate) functions: Rc<DeviceFunctions>,
}

pub struct Queue {              // 原 RhiQueue
    // Vulkan 队列封装
}

// === 中层：Vulkan 上下文管理 ===
pub struct VulkanCore {         // 原 VulkanContext
    pub(crate) entry: ash::Entry,
    pub(crate) instance: Instance,
    pub(crate) physical_device: PhysicalDevice, 
    pub(crate) device: Device,              // 注意：这里使用重命名后的 Device
    pub(crate) graphics_queue: Queue,
    // ...
}

// === 高层：RHI 主入口点 ===
pub struct RenderContext {      // 原 Rhi
    pub(crate) core: VulkanCore,
    pub(crate) allocator: Rc<MemoryAllocator>,
    pub(crate) resources: RefCell<ResourceManager>,
    // ...
}
```

**向后兼容性**:
```rust
// 通过类型别名保持完全的向后兼容
pub type Rhi = RenderContext;
pub type RhiDevice = Device;
pub type RhiDeviceFunctions = DeviceFunctions;
pub type VulkanContext = VulkanCore;
```
```

### 阶段 3: 模块化重构

```rust
// 更清晰的模块结构
pub mod core {
    pub use crate::vulkan_context::VulkanCore;
    pub use crate::foundation::{Device, Instance, PhysicalDevice};
}

pub mod commands {
    pub use crate::commands::{CommandBuffer, CommandPool, Queue};
}

pub mod resources {
    pub use crate::resources_new::{ResourceManager, Buffer, Image, ImageView};
}

pub mod memory {
    pub use crate::foundation::mem_allocator::MemoryAllocator;
}
```

## 实施策略

### 步骤 1: 添加类型别名（向后兼容）
```rust
// === 在各个模块中添加类型别名 ===

// foundation/device.rs
pub type RhiDeviceFunctions = DeviceFunctions;
pub type RhiDevice = Device;

// commands/
pub type RhiCommandBuffer = CommandBuffer;
pub type RhiQueue = Queue;
pub type RhiCommandPool = CommandPool;

// rhi.rs  
pub type Rhi = RenderContext;

// vulkan_context.rs
pub type VulkanContext = VulkanCore;
```

**命名冲突解决方案**:

1. **`RenderContext` vs `RhiDevice`**: 
   - `RenderContext`: 高层抽象，管理整个渲染上下文
   - `Device` (原 `RhiDevice`): 中层抽象，Vulkan 逻辑设备封装
   - 语义完全不同，不会混淆

2. **清晰的层次结构**:
   ```
   RenderContext (高层 - 渲染上下文管理)
   ├── VulkanCore (中层 - Vulkan 对象管理)  
   │   ├── Device (中层 - 逻辑设备)
   │   │   └── DeviceFunctions (低层 - 函数指针)
   │   ├── Queue (中层 - 队列)
   │   └── ...
   ├── MemoryAllocator (高层 - 内存管理)
   └── ResourceManager (高层 - 资源管理)
   ```

### 步骤 2: 重构内部使用
逐步在 crate 内部使用新名称，但保持公共 API 兼容。

### 步骤 3: 更新文档和示例
更新所有文档使用新的命名约定。

### 步骤 4: 弃用警告
为旧名称添加 `#[deprecated]` 标记。

## 预期收益

1. **更清晰的职责分离**: `RenderDevice` vs `VulkanCore`
2. **减少命名冗余**: 在 crate 内部去除不必要的前缀
3. **更好的模块化**: 清晰的模块边界
4. **保持兼容性**: 通过类型别名保持 API 稳定

## 风险评估

- **低风险**: 通过类型别名保持向后兼容
- **中等工作量**: 需要更新内部代码和文档
- **长期收益**: 更清晰和可维护的代码库
