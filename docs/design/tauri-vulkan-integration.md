# Tauri + Vulkan 渲染器集成方案

## 概述

在 Tauri 应用中嵌入 Vulkan 渲染窗口，实现 **Web UI + 原生渲染** 混合架构。

**核心思路**：在 Tauri 主窗口内创建 Win32 子窗口作为 Vulkan Surface，通过前端布局控制子窗口位置，实现 WebView UI 与 Vulkan 渲染区域的无缝融合。

---

## 架构

```
┌─────────────────────────────────────────────────┐
│              Tauri 主窗口 (HWND)                 │
│  ┌───────────────────────────────────────────┐  │
│  │            WebView2 (全覆盖)               │  │
│  │   ┌─────┐ ┌─────────────────┐ ┌─────┐     │  │
│  │   │左栏 │ │   透明区域      │ │右栏 │     │  │
│  │   │HTML │ │ (Vulkan 可见)   │ │HTML │     │  │
│  │   └─────┘ └─────────────────┘ └─────┘     │  │
│  └───────────────────────────────────────────┘  │
│              ┌─────────────────┐                │
│              │  Win32 子窗口   │ ← Vulkan 渲染  │
│              │  (Z-Order 最高) │                │
│              └─────────────────┘                │
└─────────────────────────────────────────────────┘
```

### 模块职责

| 模块 | 职责 |
|:---|:---|
| **child_window.rs** | 创建 Win32 子窗口，捕获输入事件 |
| **render_thread.rs** | 独立渲染线程，运行 RenderApp 循环 |
| **App.tsx** | 前端布局管理，计算 Vulkan 区域边距 |
| **tauri_event_adapter.rs** | Tauri 事件 → InputEvent 转换 |

---

## 核心机制

### WebView 与 Vulkan 区域同步

**问题**：前端 UI 面板可拖拽调整大小，Vulkan 子窗口必须精确匹配中间的透明区域。

**方案**：前端维护四边边距 `{top, left, right, bottom}`，尺寸变化时通过 Tauri Command 通知后端调整子窗口。

```
React 面板拖拽 → useEffect 监听 → invoke("update_vulkan_bounds") → 后端调整子窗口位置
```

### 同步触发点

| 事件 | 处理 |
|------|------|
| 初始化 | 使用默认边距创建子窗口 |
| 面板拖拽 | 前端调用 `update_vulkan_bounds` |
| 窗口 Resize | 后端使用保存的边距重算位置 |

### 输入事件路由

子窗口在 WebView 之上，直接通过 Win32 WndProc 捕获鼠标/键盘事件，转发到渲染线程。

```
Win32 消息 → child_window_proc → 全局回调 → RenderThread::send_event
```

### 渲染线程

独立线程运行渲染循环，通过 `mpsc` channel 接收输入事件和控制消息。

```rust
loop {
    handle_messages();      // 处理输入、Resize、Shutdown
    render_app.big_update(); // 渲染一帧
}
```

---

## 关键设计

| 设计点 | 说明 |
|-------|------|
| 子窗口 Z-Order | `SetWindowPos(HWND_TOP)` 确保在 WebView 之上 |
| 跨线程句柄 | `SendableHwnd` 包装 HWND 实现 Send/Sync |
| DPI 感知 | 边距乘以 `scale_factor` 转换为物理像素 |

---

## 限制

- **仅支持 Windows**：依赖 Win32 子窗口机制
- 非 Windows 平台提供空实现占位符
