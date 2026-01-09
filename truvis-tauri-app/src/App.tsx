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
    <div className="editor-container">
      {/* 顶部工具栏 */}
      <div className="toolbar" style={{ height: topHeight }}>
        <div className="toolbar-content">
          <span className="logo">🎮 Truvis Editor</span>
          <div className="toolbar-actions">
            <button className="toolbar-btn">File</button>
            <button className="toolbar-btn">Edit</button>
            <button className="toolbar-btn">View</button>
            <button className="toolbar-btn">Help</button>
          </div>
        </div>
        <div 
          className={`resize-handle-h ${resizing === 'top' ? 'active' : ''}`}
          onMouseDown={handleMouseDown('top')}
        />
      </div>

      {/* 中间区域 */}
      <div className="middle-area">
        {/* 左侧面板 */}
        <div className="left-panel" style={{ width: leftWidth }}>
          <div className="panel-content">
            <h3>Scene</h3>
            <div className="tree-view">
              <div className="tree-item">📁 Root</div>
              <div className="tree-item indent">📦 Mesh</div>
              <div className="tree-item indent">💡 Light</div>
              <div className="tree-item indent">📷 Camera</div>
            </div>
          </div>
          <div 
            className={`resize-handle-v ${resizing === 'left' ? 'active' : ''}`}
            onMouseDown={handleMouseDown('left')}
          />
        </div>

        {/* Vulkan 渲染区域（透明占位，捕获鼠标事件） */}
        <div 
          ref={vulkanRef}
          className="vulkan-placeholder"
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
        <div className="right-panel" style={{ width: rightWidth }}>
          <div 
            className={`resize-handle-v left ${resizing === 'right' ? 'active' : ''}`}
            onMouseDown={handleMouseDown('right')}
          />
          <div className="panel-content">
            <h3>Properties</h3>
            <div className="property-group">
              <label>Position</label>
              <div className="property-row">
                <span>X: 0.0</span>
                <span>Y: 0.0</span>
                <span>Z: 0.0</span>
              </div>
            </div>
            <div className="property-group">
              <label>Rotation</label>
              <div className="property-row">
                <span>X: 0°</span>
                <span>Y: 0°</span>
                <span>Z: 0°</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* 底部状态栏 */}
      <div className="statusbar" style={{ height: bottomHeight }}>
        <div 
          className={`resize-handle-h top ${resizing === 'bottom' ? 'active' : ''}`}
          onMouseDown={handleMouseDown('bottom')}
        />
        <div className="statusbar-content">
          <span>Ready</span>
          <span className="status-right">FPS: -- | Draw Calls: -- | Triangles: --</span>
        </div>
      </div>
    </div>
  );
}

export default App;
