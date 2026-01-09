import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import "./App.css";

// 默认布局尺寸（像素）
const DEFAULT_TOP_HEIGHT = 40;
const DEFAULT_BOTTOM_HEIGHT = 24;
const DEFAULT_LEFT_WIDTH = 200;
const DEFAULT_RIGHT_WIDTH = 200;

// 最小尺寸
const MIN_PANEL_SIZE = 100;
const MIN_VULKAN_SIZE = 200;

interface VulkanBounds {
  top: number;
  left: number;
  right: number;
  bottom: number;
}

function App() {
  // 面板尺寸状态
  const [topHeight, setTopHeight] = useState(DEFAULT_TOP_HEIGHT);
  const [bottomHeight, setBottomHeight] = useState(DEFAULT_BOTTOM_HEIGHT);
  const [leftWidth, setLeftWidth] = useState(DEFAULT_LEFT_WIDTH);
  const [rightWidth, setRightWidth] = useState(DEFAULT_RIGHT_WIDTH);
  
  // 拖拽状态
  const [resizing, setResizing] = useState<string | null>(null);
  
  // Vulkan 区域引用
  const vulkanRef = useRef<HTMLDivElement>(null);

  // 通知后端更新 Vulkan 区域布局
  const updateVulkanBounds = useCallback(async (bounds: VulkanBounds) => {
    try {
      await invoke("update_vulkan_bounds", { 
        top: Math.round(bounds.top),
        left: Math.round(bounds.left),
        right: Math.round(bounds.right),
        bottom: Math.round(bounds.bottom)
      });
    } catch (error) {
      console.error("Failed to update vulkan bounds:", error);
    }
  }, []);

  // 初始化时和尺寸变化时更新后端
  useEffect(() => {
    const bounds: VulkanBounds = {
      top: topHeight,
      left: leftWidth,
      right: rightWidth,
      bottom: bottomHeight
    };
    updateVulkanBounds(bounds);
  }, [topHeight, bottomHeight, leftWidth, rightWidth, updateVulkanBounds]);

  // 窗口大小变化时也需要更新
  useEffect(() => {
    const handleResize = () => {
      const bounds: VulkanBounds = {
        top: topHeight,
        left: leftWidth,
        right: rightWidth,
        bottom: bottomHeight
      };
      updateVulkanBounds(bounds);
    };

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [topHeight, bottomHeight, leftWidth, rightWidth, updateVulkanBounds]);

  // 拖拽处理
  const handleMouseDown = useCallback((edge: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    setResizing(edge);
  }, []);

  useEffect(() => {
    if (!resizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const windowWidth = window.innerWidth;
      const windowHeight = window.innerHeight;

      switch (resizing) {
        case "top":
          setTopHeight(Math.max(30, Math.min(windowHeight - bottomHeight - MIN_VULKAN_SIZE, e.clientY)));
          break;
        case "bottom":
          setBottomHeight(Math.max(20, Math.min(windowHeight - topHeight - MIN_VULKAN_SIZE, windowHeight - e.clientY)));
          break;
        case "left":
          setLeftWidth(Math.max(MIN_PANEL_SIZE, Math.min(windowWidth - rightWidth - MIN_VULKAN_SIZE, e.clientX)));
          break;
        case "right":
          setRightWidth(Math.max(MIN_PANEL_SIZE, Math.min(windowWidth - leftWidth - MIN_VULKAN_SIZE, windowWidth - e.clientX)));
          break;
      }
    };

    const handleMouseUp = () => {
      setResizing(null);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [resizing, topHeight, bottomHeight, leftWidth, rightWidth]);

  // 获取相对于 Vulkan 区域的鼠标坐标
  const getVulkanRelativePos = useCallback((e: React.MouseEvent | MouseEvent) => {
    if (!vulkanRef.current) return { x: 0, y: 0 };
    const rect = vulkanRef.current.getBoundingClientRect();
    return {
      x: e.clientX - rect.left,
      y: e.clientY - rect.top
    };
  }, []);

  // Vulkan 区域鼠标移动事件
  const handleVulkanMouseMove = useCallback((e: React.MouseEvent) => {
    const pos = getVulkanRelativePos(e);
    emit("render:mouse_move", { x: pos.x, y: pos.y });
  }, [getVulkanRelativePos]);

  // Vulkan 区域鼠标按钮事件
  const handleVulkanMouseDown = useCallback((e: React.MouseEvent) => {
    console.log("Mouse down:", e.button);
    emit("render:mouse_button", { button: e.button, pressed: true });
    // 聚焦到 Vulkan 区域以接收键盘事件
    vulkanRef.current?.focus();
  }, []);

  const handleVulkanMouseUp = useCallback((e: React.MouseEvent) => {
    console.log("Mouse up:", e.button);
    emit("render:mouse_button", { button: e.button, pressed: false });
  }, []);

  // Vulkan 区域滚轮事件
  const handleVulkanWheel = useCallback((e: React.WheelEvent) => {
    console.log("Wheel:", e.deltaY);
    emit("render:mouse_wheel", { delta: -e.deltaY / 100 });
  }, []);

  // Vulkan 区域键盘事件（需要 focus）
  const handleVulkanKeyDown = useCallback((e: React.KeyboardEvent) => {
    console.log("Key down:", e.key);
    emit("render:keyboard", { key: e.key, pressed: true });
  }, []);

  const handleVulkanKeyUp = useCallback((e: React.KeyboardEvent) => {
    console.log("Key up:", e.key);
    emit("render:keyboard", { key: e.key, pressed: false });
  }, []);

  return (
    <div className="flex flex-col w-full h-screen bg-editor-bg">
      {/* 顶部工具栏 */}
      <div className="bg-editor-toolbar border-b border-editor-border flex flex-col shrink-0" style={{ height: topHeight }}>
        <div className="flex-1 flex items-center px-3 gap-4">
          <span className="font-semibold text-sm text-editor-text-white">🎮 Truvis Editor</span>
          <div className="flex gap-1">
            <button className="bg-transparent border-none text-editor-text-primary px-2.5 py-1 rounded cursor-pointer text-[13px] hover:bg-editor-border transition-colors">File</button>
            <button className="bg-transparent border-none text-editor-text-primary px-2.5 py-1 rounded cursor-pointer text-[13px] hover:bg-editor-border transition-colors">Edit</button>
            <button className="bg-transparent border-none text-editor-text-primary px-2.5 py-1 rounded cursor-pointer text-[13px] hover:bg-editor-border transition-colors">View</button>
            <button className="bg-transparent border-none text-editor-text-primary px-2.5 py-1 rounded cursor-pointer text-[13px] hover:bg-editor-border transition-colors">Help</button>
          </div>
        </div>
        <div 
          className={`h-1 bg-transparent cursor-ns-resize shrink-0 transition-colors hover:bg-editor-accent ${resizing === 'top' ? 'bg-editor-accent' : ''}`}
          onMouseDown={handleMouseDown('top')}
        />
      </div>

      {/* 中间区域 */}
      <div className="flex-1 flex overflow-hidden">
        {/* 左侧面板 */}
        <div className="bg-editor-panel flex shrink-0 relative" style={{ width: leftWidth }}>
          <div className="flex-1 overflow-y-auto p-2">
            <h3 className="text-[11px] font-semibold text-editor-text-secondary px-2 py-1.5 bg-[#333333] -mx-2 -mt-2 mb-2 uppercase tracking-wide border-b border-editor-border">Scene</h3>
            <div className="text-[13px]">
              <div className="px-2 py-1 cursor-pointer rounded hover:bg-editor-hover">📁 Root</div>
              <div className="px-2 py-1 cursor-pointer rounded hover:bg-editor-hover pl-6">📦 Mesh</div>
              <div className="px-2 py-1 cursor-pointer rounded hover:bg-editor-hover pl-6">💡 Light</div>
              <div className="px-2 py-1 cursor-pointer rounded hover:bg-editor-hover pl-6">📷 Camera</div>
            </div>
          </div>
          <div 
            className={`w-1 bg-transparent cursor-ew-resize shrink-0 transition-colors absolute right-0 top-0 bottom-0 hover:bg-editor-accent ${resizing === 'left' ? 'bg-editor-accent' : ''}`}
            onMouseDown={handleMouseDown('left')}
          />
        </div>

        {/* Vulkan 渲染区域（透明占位，捕获鼠标事件） */}
        <div 
          ref={vulkanRef}
          className="flex-1 bg-transparent cursor-crosshair outline-none"
          tabIndex={0}
          onMouseMove={handleVulkanMouseMove}
          onMouseDown={handleVulkanMouseDown}
          onMouseUp={handleVulkanMouseUp}
          onWheel={handleVulkanWheel}
          onKeyDown={handleVulkanKeyDown}
          onKeyUp={handleVulkanKeyUp}
          onContextMenu={(e) => e.preventDefault()}
        >
          {/* 这个区域捕获鼠标事件并转发给 Vulkan 渲染器 */}
        </div>

        {/* 右侧面板 */}
        <div className="bg-editor-panel flex shrink-0 relative" style={{ width: rightWidth }}>
          <div 
            className={`w-1 bg-transparent cursor-ew-resize shrink-0 transition-colors absolute left-0 top-0 bottom-0 hover:bg-editor-accent ${resizing === 'right' ? 'bg-editor-accent' : ''}`}
            onMouseDown={handleMouseDown('right')}
          />
          <div className="flex-1 overflow-y-auto p-2">
            <h3 className="text-[11px] font-semibold text-editor-text-secondary px-2 py-1.5 bg-[#333333] -mx-2 -mt-2 mb-2 uppercase tracking-wide border-b border-editor-border">Properties</h3>
            <div className="mb-3">
              <label className="block text-[11px] text-editor-text-muted mb-1 uppercase">Position</label>
              <div className="flex gap-2 text-xs text-editor-text-primary">
                <span>X: 0.0</span>
                <span>Y: 0.0</span>
                <span>Z: 0.0</span>
              </div>
            </div>
            <div className="mb-3">
              <label className="block text-[11px] text-editor-text-muted mb-1 uppercase">Rotation</label>
              <div className="flex gap-2 text-xs text-editor-text-primary">
                <span>X: 0°</span>
                <span>Y: 0°</span>
                <span>Z: 0°</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="bg-editor-accent flex flex-col shrink-0" style={{ height: bottomHeight }}>
        <div 
          className={`h-1 bg-transparent cursor-ns-resize shrink-0 transition-colors hover:bg-editor-accent ${resizing === 'bottom' ? 'bg-editor-accent' : ''}`}
          onMouseDown={handleMouseDown('bottom')}
        />
        <div className="flex-1 flex items-center justify-between px-3 text-xs text-editor-text-white">
          <span>Ready</span>
          <span className="opacity-80">FPS: -- | Draw Calls: -- | Triangles: --</span>
        </div>
      </div>
    </div>
  );
}

export default App;
