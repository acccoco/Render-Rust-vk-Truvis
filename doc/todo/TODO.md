# TODO 和 FIXME 任务清单

本文档整理了项目中所有的 TODO 和 FIXME 项目，按照模块和优先级进行分类。

*最后更新：2025年8月26日*

## 🚨 紧急修复 (FIXME) - 影响功能和用户体验

### 🔴 阻塞性问题
- **`truvis-render/src/app.rs:227`** - **用户事件处理未实现**
  - 问题：user_event 函数为 `todo!()`，导致程序 panic
  - 影响：**严重** - 阻塞程序运行
  - 修复优先级：**立即**

- **`truvis-render/src/app.rs:236`** - **ImGui 事件处理缺失**
  - 问题：应该接收 imgui 的事件，但当前没有正确处理
  - 影响：**高** - GUI 交互功能缺失
  - 修复优先级：**立即**

### 🟠 功能性问题
- **`shader/include/light.slangi:29`** - **光照衰减计算错误**
  - 问题：光照衰减计算需要修复 (`FIXME attenuation`)
  - 影响：**中** - 光照效果不正确，影响渲染质量
  - 修复优先级：**短期**

- **`truvis-rhi/src/core/resources/texture.rs:18`** - **UUID 功能未启用**
  - 问题：将 uuid 使用起来，当前未启用
  - 影响：**低** - 纹理资源管理缺乏唯一标识
  - 修复优先级：**中期**

## 🏗️ 架构重构任务 - 提升系统架构和可维护性

### 🔥 高优先级重构
- **Renderer 结构体重构** (`RENDERER_REFACTOR.md`)
  - 问题：过度使用 `Rc<RefCell<>>` 造成性能开销和复杂性
  - 目标：重构为分层架构 (RenderCore, RenderResources, SceneContext, RenderSettings)
  - 影响：**高** - 提升性能，降低复杂性
  - 预计工作量：**大** (2-3 周)

- **RHI 资源管理重构** (`RHI_REFACTOR_PROPOSAL.md`)
  - 问题：循环引用风险，资源管理分散
  - 目标：句柄 + 资源管理器模式
  - 影响：**高** - 消除内存泄漏风险，统一资源管理
  - 预计工作量：**大** (2-3 周)

### ⚡ 中优先级重构  
- **App 架构重构** (`APP_ARCHITECTURE_REFACTOR.md`)
  - 问题：组件间耦合度高，存在循环依赖
  - 目标：解耦 Window 和 Renderer，引入 RenderCoordinator
  - 影响：**中** - 提升架构清晰度
  - 预计工作量：**中** (1-2 周)

## 📋 功能开发任务 - 新功能实现和现有功能完善

### 🎯 渲染管线增强

#### GPU 场景管理 (`truvis-render/src/renderer/gpu_scene.rs`)
- **Line 321** - **聚光灯支持**：当前 spot_lights 字段暂时无用，需要实现聚光灯功能
  - 优先级：**中** - 增强光照系统
  - 预计工作量：**中** (3-5 天)

- **Line 323** - **聚光灯数量统计**：spot_light_count 字段暂时无用
  - 依赖：聚光灯支持功能
  - 优先级：**中**

- **Line 384** - **间接索引缓冲区优化**：对于 mesh 来说，可能不需要间接的索引 buffer
  - 问题：mesh 在 geometry buffer 中是连续的，当前设计可能过度复杂
  - 优先级：**低** - 性能优化
  - 预计工作量：**小** (1-2 天)

- **Line 520** - **Hit Group 多样化**：暂时使用同一个 hit group，需要支持不同的 hit group
  - 影响：光线追踪材质多样性
  - 优先级：**中** - 光线追踪增强
  - 预计工作量：**中** (3-5 天)

- **Line 545** - **自定义索引优化**：暂时将 instance 的 index 作为 custom index
  - 问题：需要更好的索引策略
  - 优先级：**低** - 优化策略

#### 渲染器核心优化
- **`truvis-render/src/renderer/renderer.rs:47`** - **Buffer 位置重构**
  - 问题：buffer 位置不合理，需要重新组织
  - 优先级：**中** - 代码组织优化
  - 预计工作量：**小** (1-2 天)

- **`truvis-render/src/renderer/swapchain.rs:229`** - **Suboptimal 问题修复**
  - 问题：解决 swapchain suboptimal 的问题
  - 影响：渲染质量和性能
  - 优先级：**中** - 性能优化
  - 预计工作量：**小** (1-2 天)

### 🖥️ 应用程序框构完善

#### 主应用程序 (`truvis-render/src/app.rs`)
- **Line 212** - **事件发送时机确认**：确认事件发送的最佳时机
- **Line 213** - **Timer 更新**：可以在此处更新 timer
- **Line 251** - **重绘循环优化**：是否应该手动调用 redraw，实现死循环？
  - 优先级：**低** - 架构优化

### 🎨 GUI 系统完善

#### GUI 渲染通道 (`truvis-render/src/gui/gui_pass.rs`)
- **Line 75** - **深度缓冲移除**：GUI 渲染不应该使用 depth buffer
  - 优先级：**中** - 正确性修复
  - 预计工作量：**小** (半天)

- **Line 96** - **Mesh 管理优化**：mesh 应该放在 gui pass 中管理
  - 优先级：**中** - 架构优化
  - 预计工作量：**小** (1 天)

### ⚙️ RHI 系统优化

#### 资源管理优化
- **`truvis-rhi/src/resources/managed_image.rs:122`** - **Buffer 创建方式优化**
  - 问题：使用新的 Buffer 创建方式来优化代码
  - 优先级：**低** - 代码现代化
  - 预计工作量：**小** (1 天)

- **`truvis-rhi/src/core/resources/special_buffers/structured_buffer.rs:48`** - **Flag 优化**
  - 问题：或许可以优化这个 flag
  - 优先级：**低** - 性能微调

- **`truvis-rhi/src/core/resources/special_buffers/structured_buffer.rs:63`** - **对齐优化**
  - 问题：可能不需要这个 align
  - 优先级：**低** - 性能微调

### 🎨 着色器系统增强

#### 后处理着色器
- **`shader/src/pass/pp/sdr.slang:19`** - **随机性增强**
  - 功能：引入 accum frame 的参数，增加随机性
  - 优先级：**低** - 质量提升
  - 预计工作量：**小** (1 天)

#### PBR 渲染
- **`shader/include/pbr.slangi:118`** - **未完成功能实现**
  - 问题：有未实现的功能（注释仅为 "TODO"）
  - 优先级：**中** - 功能完整性
  - 预计工作量：**需调研** - 需要确定具体功能

#### ShaderToy 实验功能
- **`shader/src/shadertoy-glsl/works/corner_box.glsl:331`** - **光线方向通用化**
  - 功能：使光线方向更通用化
  - 优先级：**极低** - 实验性功能

- **`shader/src/shadertoy-glsl/works/corner_box.glsl:348`** - **重要性采样**
  - 功能：在漫反射和镜面反射上实现多重要性采样
  - 优先级：**极低** - 实验性功能

- **`shader/src/shadertoy-glsl/works/chainsaw_man_power.glsl:164`** - **代码清理**
  - 问题：清理相关代码 (`rb?!?!?!?!?`)
  - 优先级：**极低** - 代码清理

- **`shader/src/shadertoy-glsl/works/chainsaw_man_power.glsl:645`** - **颜色一致性**
  - 问题：应该与头部阴影使用相同颜色
  - 优先级：**极低** - 美术调整

### 🔗 C++ 集成优化

#### Assimp 集成 (`truvis-cxx/src/lib.rs`)
- **Line 13** - **SoA 重构**：使用 SoA (Structure of Arrays) 来简化
  - 影响：可以移除复杂的数组处理代码
  - 优先级：**低** - 代码简化
  - 预计工作量：**中** (2-3 天)

## 📊 开发优先级矩阵

### 🔥 立即处理 (本周内)
1. **用户事件处理未实现** - 阻塞程序运行
2. **ImGui 事件处理缺失** - 影响用户交互
3. **光照衰减计算修复** - 影响渲染质量

### ⚡ 短期处理 (2-4 周内)
1. **Renderer 结构体重构** - 重要架构改进
2. **聚光灯功能实现** - 重要功能增强
3. **GUI 系统优化** - 深度缓冲和 Mesh 管理
4. **Hit Group 多样化** - 光线追踪增强

### 🔧 中期处理 (1-2 个月内)
1. **RHI 资源管理重构** - 架构稳固化
2. **App 架构重构** - 降低耦合度
3. **Buffer 和交换链优化** - 性能提升
4. **PBR 功能完善** - 功能完整性

### 📈 长期处理 (2+ 个月内)
1. **RHI 系统细节优化** - 性能微调
2. **着色器后处理增强** - 质量提升  
3. **C++ 集成重构** - 代码现代化
4. **ShaderToy 实验功能** - 探索性开发

## 🎯 里程碑规划

### 阶段 1：紧急修复 (Week 1)
- [ ] 修复用户事件处理 panic
- [ ] 实现 ImGui 事件处理
- [ ] 修复光照衰减计算

### 阶段 2：核心重构 (Week 2-5)  
- [ ] Renderer 结构体重构
- [ ] 聚光灯功能实现
- [ ] GUI 系统优化

### 阶段 3：架构强化 (Week 6-10)
- [ ] RHI 资源管理重构
- [ ] App 架构解耦
- [ ] 光线追踪增强

### 阶段 4：性能优化 (Week 11-16)
- [ ] 缓冲区和索引优化
- [ ] 交换链质量提升
- [ ] PBR 功能完善

## 📋 任务分配建议

### 核心开发者
- 架构重构任务 (Renderer, RHI, App)
- 复杂功能实现 (聚光灯, Hit Group)
- 关键性能优化

### 贡献者
- GUI 系统优化
- 着色器功能增强
- 代码清理和文档

### 新手友好任务 🌟
- ShaderToy 功能调整
- 代码注释完善
- 简单的 Flag/对齐优化
- 文档更新

## 📝 开发注意事项

### 重构期间策略
- **避免同时进行多个大型重构**，优先完成 Renderer 重构
- **保持向后兼容性**，渐进式迁移
- **充分测试每个阶段**，确保稳定性

### 代码质量要求
- 每个 TODO 修复后需要添加相应测试
- 重构需要更新相关文档
- 性能敏感的修改需要 benchmark 验证

### 协作建议
- 大型重构前需要 RFC (Request for Comments)
- 关键架构变更需要团队讨论
- 定期同步重构进度，避免冲突

---

*本文档将根据开发进度定期更新。建议每完成一个里程碑后重新评估优先级。*

## 📋 补充说明

### 重构文档参考
- **详细重构方案**: 参见 `doc/todo/RENDERER_REFACTOR.md`
- **App 架构设计**: 参见 `doc/todo/APP_ARCHITECTURE_REFACTOR.md`  
- **RHI 重构提案**: 参见 `doc/todo/RHI_REFACTOR_PROPOSAL.md`

### 代码贡献指南
- 修复 TODO 前请先查看相关重构文档
- 大型架构变更需要在重构文档中讨论
- 优先修复阻塞性和高影响问题
- 每个修复都应包含相应的测试和文档更新

### 性能和质量要求
- 性能敏感的修改需要 benchmark 验证
- 重构后的代码应该有更好的可维护性
- 保持向后兼容性，渐进式迁移

---
*最后更新：2025年8月26日 - 基于代码分析和重构文档综合整理*
