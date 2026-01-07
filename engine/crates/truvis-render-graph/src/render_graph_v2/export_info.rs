use crate::render_graph_v2::RgImageState;
use crate::render_graph_v2::semaphore_info::RgSemaphoreInfo;

/// 导出资源信息
///
/// 描述资源在渲染图执行完成后的最终状态和同步需求。
#[derive(Clone, Debug)]
pub struct RgExportInfo {
    /// 资源的最终状态（layout, access, stage）
    pub final_state: RgImageState,
    /// 可选的信号 semaphore
    pub signal_semaphore: Option<RgSemaphoreInfo>,
}

impl RgExportInfo {
    /// 创建导出信息（无 semaphore）
    #[inline]
    pub fn new(final_state: RgImageState) -> Self {
        Self {
            final_state,
            signal_semaphore: None,
        }
    }

    /// 创建导出信息（带 semaphore）
    #[inline]
    pub fn with_signal(final_state: RgImageState, signal_semaphore: RgSemaphoreInfo) -> Self {
        Self {
            final_state,
            signal_semaphore: Some(signal_semaphore),
        }
    }
}
