use crate::handle::AssetTextureHandle;
use ash::vk;
use crossbeam_channel::{Receiver, Sender};
use crossbeam_utils::sync::WaitGroup;
use image::GenericImageView;
use std::path::PathBuf;
use std::thread;

pub struct AssetLoadRequest {
    pub path: PathBuf,
    pub handle: AssetTextureHandle,
    // pub params: AssetParams, // Future expansion
}

/// 解码后的原始资产数据 (CPU 端)
/// 准备好上传到 GPU
pub struct RawAssetData {
    pub pixels: Vec<u8>,
    pub extent: vk::Extent3D,
    pub format: vk::Format,
    pub handle: AssetTextureHandle,
    pub mip_levels: u32,
}

pub enum LoadResult {
    Success(RawAssetData),
    Failure(AssetTextureHandle, String),
}

// TODO 一句话解释：这就是线程池：crossbeam 提供消息队列；rayon 提供线程池
/// IO 工作器
///
/// 负责管理后台 IO 任务。
/// 架构设计:
/// 1. 主线程通过 `request_tx` 发送加载请求。
/// 2. 内部有一个 "Asset-IO-Dispatcher" 线程不断接收请求。
/// 3. Dispatcher 将繁重的 IO 和解码任务 (image::open) 派发给 `rayon` 全局线程池。
/// 4. 完成的任务通过 `result_rx` 发送回主线程 (AssetHub)。
///
/// 这种设计确保了:
/// - 主线程不会被 IO 阻塞 (非阻塞发送)。
/// - 繁重的图片解码利用多核 CPU (rayon)。
/// - 结果处理在主线程统一进行 (AssetHub::update)。
///
/// # 线程生命周期
/// "Asset-IO-Dispatcher" 线程的生命周期与 `IoWorker` 实例绑定。
/// 当 `IoWorker` 被 Drop 时：
/// 1. `request_tx` 被销毁，导致 channel 断开。
/// 2. 后台线程中的 `req_rx.recv()` 返回错误，退出循环。
/// 3. 线程执行 `wg.wait()`，阻塞等待所有已分发的 Rayon 任务完成。
/// 4. `IoWorker::drop` 会调用 `thread.join()` 等待后台线程完全退出。
pub struct IoDispather {
    /// 用于向 IoWorker 发送加载请求
    request_sender: Option<Sender<AssetLoadRequest>>,
    /// 用于从 IoWorker 接收加载结果
    result_receiver: Receiver<LoadResult>,

    /// 用于分发 IO 任务的后台线程
    dispatch_thread: Option<std::thread::JoinHandle<()>>,
}

impl Default for IoDispather {
    fn default() -> Self {
        Self::new()
    }
}

impl IoDispather {
    pub fn new() -> Self {
        let (req_tx, req_rx) = crossbeam_channel::unbounded::<AssetLoadRequest>();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<LoadResult>();

        // 创建一个专用的 Rayon 线程池，并设置线程名称
        // 这样在调试器中可以看到 "Asset-Loader-0", "Asset-Loader-1" 等
        let pool = rayon::ThreadPoolBuilder::new()
            .thread_name(|index| format!("Asset-Loader-{}", index))
            .build()
            .expect("Failed to create asset loader thread pool");

        // 启动一个协调线程，负责从 channel 接收请求并分发给 rayon 线程池
        // 这是一个轻量级线程，只负责消息转发，不进行重计算
        let io_thread = thread::Builder::new()
            .name("Asset-IO-Dispatcher".to_string())
            .spawn(move || {
                let wg = WaitGroup::new();

                while let Ok(req) = req_rx.recv() {
                    let _span = tracy_client::span!("IoWorker::dispatch");

                    let res_tx = res_tx.clone();
                    // 为每个任务克隆一个 WaitGroup
                    // 当任务结束，闭包销毁，wg_task 也会被 drop
                    let wg_task = wg.clone();

                    // 使用专用线程池执行任务
                    pool.spawn(move || {
                        let result = load_texture_task(req);
                        let _ = res_tx.send(result);

                        // wg_task 在这里自动 drop
                        drop(wg_task);
                    });
                }

                // 等待所有任务完成
                wg.wait();
            })
            .expect("Failed to spawn IO dispatcher thread");

        Self {
            request_sender: Some(req_tx),
            result_receiver: res_rx,

            dispatch_thread: Some(io_thread),
        }
    }

    pub fn request_load(&self, req: AssetLoadRequest) {
        if let Some(sender) = &self.request_sender
            && let Err(e) = sender.send(req)
        {
            log::error!("Failed to send asset load request: {}", e);
        }
    }

    pub fn try_recv_result(&self) -> Option<LoadResult> {
        self.result_receiver.try_recv().ok()
    }

    /// 显式等待所有任务完成并销毁 IoWorker
    /// 实际上只是消耗 self，触发 Drop
    pub fn join(self) {}
}

impl Drop for IoDispather {
    fn drop(&mut self) {
        // 显式关闭 channel，通知后台线程退出
        // 必须先 drop sender，否则 recv 会一直阻塞，导致 join 死锁
        self.request_sender = None;

        log::info!("IoWorker is being dropped, waiting for tasks to complete...");
        if let Some(thread) = self.dispatch_thread.take()
            && let Err(_) = thread.join()
        {
            log::error!("Failed to join IO dispatcher thread");
        }
        log::info!("All IO tasks completed, IoWorker dropped.");
    }
}

/// 实际的加载任务 (运行在 Rayon 线程池中)
/// 执行: 文件读取 -> 图片解码 -> 格式转换
fn load_texture_task(req: AssetLoadRequest) -> LoadResult {
    let _span = tracy_client::span!("load_texture_task");
    log::info!("Loading texture: {:?}", req.path);

    let img_result = image::open(&req.path);

    match img_result {
        Ok(img) => {
            let (width, height) = img.dimensions();
            // 强制转换为 RGBA8
            let img = img.into_rgba8();
            let pixels = img.into_raw();

            let raw_data = RawAssetData {
                pixels,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                format: vk::Format::R8G8B8A8_UNORM, // 目前统一转为 RGBA8
                handle: req.handle,
                mip_levels: 1, // 暂时只加载 level 0
            };

            LoadResult::Success(raw_data)
        }
        Err(e) => {
            log::error!("Failed to load texture {:?}: {}", req.path, e);
            LoadResult::Failure(req.handle, e.to_string())
        }
    }
}
