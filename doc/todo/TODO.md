# TODO 和 FIXME 任务清单

本文档整理了项目中所有的 TODO 和 FIXME 项目，按照模块和优先级进行分类。

## 🚨 高优先级 (FIXME)

### 渲染系统
- **`truvis-render/src/app.rs:236`** - ImGui 事件处理
  - 问题：应该接收 imgui 的事件，但当前没有正确处理
  - 影响：GUI 交互可能存在问题

### RHI 系统  
- **`truvis-rhi/src/core/resources/texture.rs:18`** - UUID 使用
  - 问题：将 uuid 使用起来，当前未启用
  - 影响：纹理资源管理可能缺乏唯一标识

### 着色器系统
- **`shader/include/light.slangi:29`** - 光照衰减计算
  - 问题：光照衰减计算需要修复
  - 影响：光照效果可能不正确

## 📋 功能实现 (TODO)

### 渲染管线优化

#### GPU 场景管理 (`truvis-render/src/renderer/gpu_scene.rs`)
- **Line 321** - 聚光灯支持：当前 spot_lights 字段暂时无用，需要实现聚光灯功能
- **Line 323** - 聚光灯数量：spot_light_count 字段暂时无用  
- **Line 384** - 间接索引缓冲区优化：对于 mesh 来说，可能不需要间接的索引 buffer，因为 mesh 在 geometry buffer 中是连续的
- **Line 520** - Hit Group 多样化：暂时使用同一个 hit group，需要支持不同的 hit group
- **Line 545** - 自定义索引优化：暂时将 instance 的 index 作为 custom index，需要更好的索引策略

#### 渲染器核心 (`truvis-render/src/renderer/renderer.rs`)
- **Line 47** - Buffer 重构：优化 buffer 位置，不该放在当前位置

#### 交换链管理 (`truvis-render/src/renderer/swapchain.rs`)
- **Line 229** - Suboptimal 问题：解决 suboptimal 的问题，提升渲染质量

### 应用程序框架

#### 主应用 (`truvis-render/src/app.rs`)
- **Line 212** - 事件发送时机：确认一下发送时机
- **Line 213** - Timer 更新：可以在此处更新 timer
- **Line 227** - 用户事件处理：user_event 函数未实现 (`todo!()`)
- **Line 251** - 重绘循环：是否应该手动调用 redraw，实现死循环？

### GUI 系统

#### GUI 渲染通道 (`truvis-render/src/gui/gui_pass.rs`)
- **Line 75** - 深度缓冲：这里不应该有 depth
- **Line 96** - Mesh 管理：mesh 应该放在 gui pass 中管理

### RHI 系统优化

#### 资源管理
- **`truvis-rhi/src/resources/managed_image.rs:122`** - Buffer 创建优化：使用新的 Buffer 创建方式来优化代码
- **`truvis-rhi/src/core/resources/special_buffers/structured_buffer.rs:48`** - Flag 优化：或许可以优化这个 flag
- **`truvis-rhi/src/core/resources/special_buffers/structured_buffer.rs:63`** - 对齐优化：可能不需要这个 align

### 着色器系统

#### 后处理 (`shader/src/pass/pp/sdr.slang`)
- **Line 19** - 随机性增强：引入 accum frame 的参数，增加随机性

#### PBR 渲染 (`shader/include/pbr.slangi`)
- **Line 118** - 未完成功能：有未实现的功能（注释仅为 "TODO"）

#### ShaderToy 实现
- **`shader/src/shadertoy-glsl/works/corner_box.glsl:331`** - 通用化改进：使光线方向更通用化
- **`shader/src/shadertoy-glsl/works/corner_box.glsl:348`** - 重要性采样：在漫反射和镜面反射上实现多重要性采样
- **`shader/src/shadertoy-glsl/works/chainsaw_man_power.glsl:164`** - 代码清理：清理相关代码 (`rb?!?!?!?!?`)
- **`shader/src/shadertoy-glsl/works/chainsaw_man_power.glsl:645`** - 颜色一致性：应该与头部阴影使用相同颜色

### C++ 集成

#### Assimp 集成 (`truvis-cxx/src/lib.rs`)
- **Line 13** - SoA 重构：使用 SoA (Structure of Arrays) 来简化，就可以移除这些代码

## 📊 优先级建议

### 🔥 立即处理
1. ImGui 事件处理 (影响用户交互)
2. 光照衰减计算修复 (影响渲染质量)
3. 用户事件处理实现 (功能缺失)

### ⚡ 短期处理
1. GPU 场景管理优化
2. Buffer 位置重构
3. GUI 系统完善
4. 交换链 suboptimal 问题

### 🔧 中期处理
1. 聚光灯功能实现
2. RHI 系统优化
3. 着色器功能完善
4. C++ 集成重构

### 📈 长期处理
1. ShaderToy 功能增强
2. 渲染管线整体优化
3. 性能优化相关项目

## 📝 备注

- 本文档基于代码分析生成，建议定期更新
- 优先级评估基于功能影响范围和用户体验
- 建议在处理每个 TODO 后更新相应的文档和测试
- 某些 TODO 可能需要架构层面的讨论才能确定最佳实现方案

---
*最后更新：2025年8月26日*
