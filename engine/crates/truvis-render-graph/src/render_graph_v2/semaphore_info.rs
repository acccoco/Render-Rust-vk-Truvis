use ash::vk;

// TODO RgSemaphoreInfo 可以考虑提升到 Gfx 里面去
#[derive(Clone, Copy, Debug)]
pub struct RgSemaphoreInfo {
    /// Vulkan semaphore 原始句柄
    pub semaphore: vk::Semaphore,
    /// 等待的 pipeline stage
    pub stage: vk::PipelineStageFlags2,
    /// Timeline semaphore 的等待值（binary semaphore 为 None）
    pub value: Option<u64>,
}

impl RgSemaphoreInfo {
    /// 创建 binary semaphore 等待
    #[inline]
    pub fn binary(semaphore: vk::Semaphore, stage: vk::PipelineStageFlags2) -> Self {
        Self {
            semaphore,
            stage,
            value: None,
        }
    }

    /// 创建 timeline semaphore 等待
    #[inline]
    pub fn timeline(semaphore: vk::Semaphore, stage: vk::PipelineStageFlags2, value: u64) -> Self {
        Self {
            semaphore,
            stage,
            value: Some(value),
        }
    }
}

/// 外部 semaphore 等待信息
///
/// 用于声明导入资源需要等待的外部信号。
/// 在渲染图执行时，会将此信息添加到 queue submit 中。
#[derive(Clone, Copy, Debug)]
pub struct RgSemaphoreWait {
    pub info: RgSemaphoreInfo,
}

/// 外部 semaphore 信号信息
///
/// 用于声明导出资源完成后需要发出的信号。
/// 在渲染图执行时，会将此信息添加到 queue submit 中。
#[derive(Clone, Copy, Debug)]
pub struct RgSemaphoreSignal {
    pub info: RgSemaphoreInfo,
}
