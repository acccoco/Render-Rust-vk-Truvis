use slotmap::new_key_type;

// 强类型句柄
// 使用 slotmap 的 generational index 机制，避免悬垂引用和资源释放后的 ID 复用问题。
// 句柄本身只是一个轻量级的 ID (u64)，可以安全地 Copy/Clone。
new_key_type! { pub struct TextureHandle; }
new_key_type! { pub struct MeshHandle; }

/// 资源加载状态机
///
/// 状态流转: Unloaded -> Loading -> Uploading -> Ready
///                                  \-> Failed
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LoadStatus {
    /// 初始状态，资源尚未请求加载
    Unloaded,
    /// IO 阶段：正在后台线程读取文件或进行 CPU 解码 (如 png -> rgba8)
    Loading,
    /// GPU 上传阶段：数据已提交到 Transfer Queue，正在等待 Timeline Semaphore 信号
    /// 此时资源在 GPU 上还不可用，但 CPU 端工作已完成
    Uploading,
    /// 完成状态：GPU 资源已完全就绪，可以绑定到 Descriptor Set 进行渲染
    Ready,
    /// 失败状态：文件不存在、格式错误或解码失败
    Failed,
}
