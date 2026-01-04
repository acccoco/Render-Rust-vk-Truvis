# RenderGraph 核心设计思路

## 概述

RenderGraph 是声明式渲染管线编排系统，自动处理 Vulkan 同步（barrier）和资源生命周期。

## 四阶段流程

### 1. Pass 添加：声明依赖

- **Pass 添加顺序非常重要**：这是渲染管线的逻辑顺序，由用户决定
- 每个 Pass 通过 `setup()` 声明资源依赖：`read_image(handle, state)` / `write_image(handle, state)`

### 2. 依赖图构建：模拟资源访问

**方法**：模拟 Pass 添加顺序，跟踪资源访问，建立依赖边

```
图结构:  Node = Pass,  Edge = 资源依赖 (images[], buffers[])
```

**边建立规则**：维护 `last_writer[resource] = pass_idx`

| 依赖类型 | 规则 |
|---------|------|
| 写后读 (RAW) | Reader 依赖 Writer |
| 写后写 (WAW) | 后 Writer 依赖前 Writer |

### 3. 拓扑排序：确定执行顺序

- 对 DAG 执行拓扑排序
- 检测循环依赖（有环则 panic）
- 保证 Producer 在 Consumer 之前

### 4. Barrier 计算：模拟命令提交

**方法**：按拓扑排序后的顺序，跟踪资源状态变化，在状态转换处插入 barrier

```
for pass in execution_order:
    for (resource, required_state) in pass.resources:
        current_state = state_tracker[resource]
        if needs_barrier(current_state, required_state):
            emit_barrier(current → required)
        state_tracker[resource] = required_state
```

**Barrier 判断**：
- Layout 不同 → barrier
- 有写操作 → barrier
- 只读→只读 + 相同 layout → 跳过

## 关键设计

| 设计点 | 说明 |
|-------|------|
| 两阶段跟踪 | 图构建跟踪 "谁写了什么"；Barrier 计算跟踪 "资源当前状态" |
| 编译/执行分离 | `compile()` 生成 `CompiledGraph`，可多帧复用 |
| 两层 Handle | `RgImageHandle`（虚拟）→ `GfxImageHandle`（物理） |

## 参考

- [Frostbite FrameGraph (GDC 2017)](https://www.gdcvault.com/play/1024612/FrameGraph-Extensible-Rendering-Architecture-in)
