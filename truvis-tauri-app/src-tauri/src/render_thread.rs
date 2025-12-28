use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use truvis_app::outer_app::OuterApp;
use truvis_app::platform::input_event::InputEvent;
use truvis_app::render_app::RenderApp;

/// 包装 RawDisplayHandle 使其可以跨线程发送
/// 注意：在 Windows 上，窗口句柄可以安全地在线程间传递
/// 但必须确保在正确的线程上进行操作
#[derive(Clone, Copy)]
pub struct SendableDisplayHandle(RawDisplayHandle);

unsafe impl Send for SendableDisplayHandle {}
unsafe impl Sync for SendableDisplayHandle {}

impl SendableDisplayHandle {
    pub fn new(handle: RawDisplayHandle) -> Self {
        Self(handle)
    }

    pub fn raw(&self) -> RawDisplayHandle {
        self.0
    }
}

/// 包装 RawWindowHandle 使其可以跨线程发送
#[derive(Clone, Copy)]
pub struct SendableWindowHandle(RawWindowHandle);

unsafe impl Send for SendableWindowHandle {}
unsafe impl Sync for SendableWindowHandle {}

impl SendableWindowHandle {
    pub fn new(handle: RawWindowHandle) -> Self {
        Self(handle)
    }

    pub fn raw(&self) -> RawWindowHandle {
        self.0
    }
}

/// 渲染线程控制消息
pub enum RenderThreadMessage {
    /// 输入事件
    InputEvent(InputEvent),
    /// 初始化窗口（在窗口创建后调用）
    InitWindow {
        raw_display_handle: SendableDisplayHandle,
        raw_window_handle: SendableWindowHandle,
        scale_factor: f64,
    },
    /// 退出渲染线程
    Shutdown,
}

/// 渲染线程句柄
pub struct RenderThread {
    /// 消息发送端
    sender: Sender<RenderThreadMessage>,
    /// 线程句柄
    thread_handle: Option<JoinHandle<()>>,
    /// 运行标志
    running: Arc<AtomicBool>,
}

impl RenderThread {
    /// 创建并启动渲染线程
    ///
    /// # Arguments
    /// * `raw_display_handle` - 显示句柄（用于初始化 Vulkan）
    /// * `outer_app_factory` - 创建 OuterApp 的工厂函数
    pub fn spawn<F>(raw_display_handle: RawDisplayHandle, outer_app_factory: F) -> Self
    where
        F: FnOnce() -> Box<dyn OuterApp> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel::<RenderThreadMessage>();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let sendable_display_handle = SendableDisplayHandle::new(raw_display_handle);

        let thread_handle = thread::Builder::new()
            .name("RenderThread".to_string())
            .spawn(move || {
                Self::render_thread_main(sendable_display_handle, outer_app_factory, receiver, running_clone);
            })
            .expect("Failed to spawn render thread");

        Self {
            sender,
            thread_handle: Some(thread_handle),
            running,
        }
    }

    /// 发送输入事件到渲染线程
    pub fn send_event(&self, event: InputEvent) {
        if self.running.load(Ordering::SeqCst) {
            let _ = self.sender.send(RenderThreadMessage::InputEvent(event));
        }
    }

    /// 发送窗口初始化消息
    pub fn init_window(
        &self,
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
        scale_factor: f64,
    ) {
        let _ = self.sender.send(RenderThreadMessage::InitWindow {
            raw_display_handle: SendableDisplayHandle::new(raw_display_handle),
            raw_window_handle: SendableWindowHandle::new(raw_window_handle),
            scale_factor,
        });
    }

    /// 请求渲染线程关闭
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.sender.send(RenderThreadMessage::Shutdown);
    }

    /// 等待渲染线程结束并清理资源
    pub fn join(mut self) {
        self.shutdown();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// 检查渲染线程是否仍在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 渲染线程主函数
    fn render_thread_main<F>(
        sendable_display_handle: SendableDisplayHandle,
        outer_app_factory: F,
        receiver: Receiver<RenderThreadMessage>,
        running: Arc<AtomicBool>,
    ) where
        F: FnOnce() -> Box<dyn OuterApp>,
    {
        // 初始化环境
        RenderApp::init_env();

        let raw_display_handle = sendable_display_handle.raw();

        // 创建 OuterApp
        let outer_app = outer_app_factory();

        // 创建 RenderApp（此时还没有窗口，只初始化 Vulkan 实例）
        let mut render_app = RenderApp::new(raw_display_handle, outer_app);

        // 等待窗口初始化消息
        let mut window_initialized = false;

        while running.load(Ordering::SeqCst) {
            // 非阻塞地尝试接收消息
            match receiver.try_recv() {
                Ok(message) => match message {
                    RenderThreadMessage::InputEvent(event) => {
                        if window_initialized {
                            render_app.handle_event(&event);
                        }
                    }
                    RenderThreadMessage::InitWindow {
                        raw_display_handle,
                        raw_window_handle,
                        scale_factor,
                    } => {
                        render_app.init_after_window(raw_display_handle.raw(), raw_window_handle.raw(), scale_factor);
                        window_initialized = true;
                        println!("Render thread: Window initialized");
                    }
                    RenderThreadMessage::Shutdown => {
                        println!("Render thread: Received shutdown signal");
                        break;
                    }
                },
                Err(mpsc::TryRecvError::Empty) => {
                    // 没有消息，继续渲染循环
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // 通道断开，退出
                    println!("Render thread: Channel disconnected");
                    break;
                }
            }

            // 如果窗口已初始化，执行渲染更新
            if window_initialized {
                render_app.big_update();
            } else {
                // 窗口未初始化时，短暂休眠以避免忙等待
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        // 清理资源
        println!("Render thread: Cleaning up...");
        render_app.destroy();
        println!("Render thread: Exited");
    }
}

impl Drop for RenderThread {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
