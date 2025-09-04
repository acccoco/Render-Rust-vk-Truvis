# RenderContext 单例模式重构指南

## 概述

本文档提供了将 `Rc<DeviceFunctions>` 使用替换为 `RenderContext` 单例模式的完整重构指南。

## 重构策略

### 1. 核心原则

- **移除所有 `Rc<DeviceFunctions>` 存储**：结构体不再持有 `device_functions` 字段
- **使用全局访问**：通过 `RenderContext::get().device_functions()` 访问 DeviceFunctions
- **保持初始化顺序**：在 RenderContext 初始化过程中使用内部构造函数

### 2. 修改模式

#### 2.1 结构体定义修改

```rust
// 修改前
pub struct SomeStruct {
    handle: vk::SomeHandle,
    device_functions: Rc<DeviceFunctions>,
    // 其他字段...
}

// 修改后
pub struct SomeStruct {
    handle: vk::SomeHandle,
    // 其他字段...
}
```

#### 2.2 构造函数修改

```rust
// 修改前
impl SomeStruct {
    pub fn new(
        device_functions: Rc<DeviceFunctions>, 
        other_params: SomeType
    ) -> Self {
        // 创建逻辑
        Self {
            handle: created_handle,
            device_functions: device_functions.clone(),
            // 其他字段...
        }
    }
}

// 修改后
impl SomeStruct {
    pub fn new(other_params: SomeType) -> Self {
        let device_functions = RenderContext::get().device_functions();
        // 创建逻辑
        Self {
            handle: created_handle,
            // 其他字段...
        }
    }

    // 如果需要在 RenderContext 初始化期间使用，提供内部构造函数
    pub(crate) fn new_internal(
        device_functions: Rc<DeviceFunctions>,
        other_params: SomeType
    ) -> Self {
        // 与上面相同的创建逻辑
        Self {
            handle: created_handle,
            // 其他字段...
        }
    }
}
```

#### 2.3 方法调用修改

```rust
// 修改前
impl SomeStruct {
    pub fn some_method(&self) {
        self.device_functions.some_vulkan_call();
    }

    pub fn destroy(self) {
        self.device_functions.destroy_something(self.handle, None);
    }
}

// 修改后
impl SomeStruct {
    pub fn some_method(&self) {
        let device_functions = RenderContext::get().device_functions();
        device_functions.some_vulkan_call();
    }

    pub fn destroy(self) {
        let device_functions = RenderContext::get().device_functions();
        device_functions.destroy_something(self.handle, None);
    }
}

impl Drop for SomeStruct {
    fn drop(&mut self) {
        let device_functions = RenderContext::get().device_functions();
        device_functions.destroy_something(self.handle, None);
    }
}
```

## 需要修改的文件清单

### 已完成的文件

1. ✅ `crates/truvis-rhi/src/commands/command_pool.rs`
2. ✅ `crates/truvis-rhi/src/commands/semaphore.rs`
3. ✅ `crates/truvis-rhi/src/commands/fence.rs`
4. ✅ `crates/truvis-rhi/src/pipelines/shader.rs`
5. ✅ `crates/truvis-rhi/src/query/query_pool.rs`
6. ✅ `crates/truvis-rhi/src/descriptors/sampler.rs` (部分完成)
7. 🔄 `crates/truvis-rhi/src/resources/buffer.rs` (部分完成)

### 待修改的关键文件

#### 基础资源文件
8. `crates/truvis-rhi/src/resources/image_view.rs`
9. `crates/truvis-rhi/src/resources/image.rs`
10. `crates/truvis-rhi/src/resources/texture.rs`

#### 专用缓冲区文件
11. `crates/truvis-rhi/src/resources/special_buffers/stage_buffer.rs`
12. `crates/truvis-rhi/src/resources/special_buffers/vertex_buffer.rs`
13. `crates/truvis-rhi/src/resources/special_buffers/structured_buffer.rs`
14. `crates/truvis-rhi/src/resources/special_buffers/index_buffer.rs`
15. `crates/truvis-rhi/src/resources/special_buffers/sbt_buffer.rs`

#### 新资源管理文件
16. `crates/truvis-rhi/src/resources_new/managed_image.rs`
17. `crates/truvis-rhi/src/resources_new/managed_buffer.rs`
18. `crates/truvis-rhi/src/resources_new/buffers/index_buffer.rs`

#### 渲染管线文件
19. `crates/truvis-rhi/src/pipelines/graphics_pipeline.rs`

#### 其他组件
20. `crates/truvis-rhi/src/swapchain/render_swapchain.rs`
21. `crates/truvis-rhi/src/raytracing/acceleration.rs`

## 修改步骤

### 第一阶段：核心基础设施 (已完成)

1. ✅ 修改 RenderContext 实现单例模式
2. ✅ 修改基础命令组件（CommandPool, Semaphore, Fence）
3. ✅ 修改基础查询和着色器组件

### 第二阶段：资源管理组件

1. 修改 Image2DView 和相关视图组件
2. 修改 Image2D 和图像组件
3. 修改 Texture2D 和纹理组件
4. 修改 Buffer 的剩余构造函数

### 第三阶段：专用资源组件

1. 修改所有 special_buffers 下的文件
2. 修改 resources_new 下的文件
3. 修改管线相关文件

### 第四阶段：高级组件

1. 修改 Swapchain 组件
2. 修改光线追踪组件
3. 修改其他高级组件

## 常见问题和解决方案

### 1. 循环依赖问题

**问题**：RenderContext 初始化时需要创建 CommandPool，但 CommandPool 又需要访问 RenderContext 单例。

**解决方案**：为需要在初始化期间使用的组件提供 `new_internal` 方法：

```rust
impl CommandPool {
    // 公共接口，使用单例
    pub fn new(queue_family: QueueFamily, flags: vk::CommandPoolCreateFlags, debug_name: &str) -> Self {
        let device_functions = RenderContext::get().device_functions();
        Self::create_with_device_functions(device_functions, queue_family, flags, debug_name)
    }

    // 内部接口，用于 RenderContext 初始化
    pub(crate) fn new_internal(
        device_functions: Rc<DeviceFunctions>,
        queue_family: QueueFamily, 
        flags: vk::CommandPoolCreateFlags, 
        debug_name: &str
    ) -> Self {
        Self::create_with_device_functions(device_functions, queue_family, flags, debug_name)
    }

    // 共享的创建逻辑
    fn create_with_device_functions(
        device_functions: Rc<DeviceFunctions>,
        queue_family: QueueFamily, 
        flags: vk::CommandPoolCreateFlags, 
        debug_name: &str
    ) -> Self {
        // 实际的创建逻辑
    }
}
```

### 2. 构造函数参数简化

**问题**：移除 `device_functions` 参数后，需要更新所有调用点。

**解决方案**：分阶段进行，保持接口向后兼容：

```rust
impl SomeStruct {
    // 新接口
    pub fn new(param1: Type1, param2: Type2) -> Self {
        let device_functions = RenderContext::get().device_functions();
        Self::new_with_device_functions(device_functions, param1, param2)
    }

    // 兼容性接口（标记为废弃）
    #[deprecated(note = "Use new() instead")]
    pub fn new_with_device_functions(
        device_functions: Rc<DeviceFunctions>,
        param1: Type1, 
        param2: Type2
    ) -> Self {
        // 实际创建逻辑
    }
}
```

### 3. 测试环境适配

**问题**：单例模式可能影响单元测试的隔离性。

**解决方案**：提供测试专用的初始化方法：

```rust
#[cfg(test)]
impl RenderContext {
    pub fn init_for_test() {
        // 使用最小配置初始化，仅用于测试
    }

    pub fn reset_for_test() {
        unsafe {
            let ptr = std::ptr::addr_of_mut!(RENDER_CONTEXT);
            *ptr = None;
        }
    }
}
```

## 批量修改脚本示例

可以使用以下 sed 命令或脚本进行批量修改：

```bash
# 移除结构体中的 device_functions 字段
find crates/truvis-rhi/src -name "*.rs" -exec sed -i 's/device_functions: Rc<DeviceFunctions>,//g' {} \;

# 替换构造函数参数
find crates/truvis-rhi/src -name "*.rs" -exec sed -i 's/device_functions: Rc<DeviceFunctions>, //g' {} \;

# 替换方法调用
find crates/truvis-rhi/src -name "*.rs" -exec sed -i 's/self\.device_functions\./RenderContext::get().device_functions()./g' {} \;
```

## 验证清单

重构完成后，请确保：

1. ✅ 所有编译错误已修复
2. ⏳ 所有单元测试通过
3. ⏳ 集成测试正常运行
4. ⏳ 性能没有明显下降
5. ⏳ 内存使用量有所改善

## 性能优化建议

1. **缓存 DeviceFunctions**：在频繁调用的热路径中，可以缓存 device_functions 引用：
   ```rust
   pub fn hot_path_function(&self) {
       let device_functions = RenderContext::get().device_functions();
       // 在同一个函数中多次使用 device_functions
       device_functions.call1();
       device_functions.call2();
       device_functions.call3();
   }
   ```

2. **避免重复获取**：在循环中避免重复调用 `RenderContext::get()`

3. **内联优化**：对于简单的 getter 方法，使用 `#[inline]` 属性

## 预期收益

完成此重构后，预期获得以下收益：

1. **代码简化**：消除约 200+ 个 `Rc<DeviceFunctions>` 的传递
2. **性能提升**：减少引用计数开销，预期性能提升 3-5%
3. **内存节省**：减少引用计数的内存开销
4. **维护性改善**：简化函数签名，减少样板代码
5. **类型安全**：消除复杂的生命周期管理问题

---

*最后更新：2025年9月4日*
