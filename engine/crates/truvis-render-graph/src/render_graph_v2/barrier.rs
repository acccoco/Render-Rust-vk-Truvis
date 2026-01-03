//! Barrier 自动计算
//!
//! 根据资源状态转换自动生成 ImageMemoryBarrier 和 BufferMemoryBarrier。

use ash::vk;
use truvis_gfx::commands::barrier::{GfxBufferBarrier, GfxImageBarrier};

use super::state::{BufferState, ImageState};

/// 图像 Barrier 描述
#[derive(Clone, Debug)]
pub struct ImageBarrierDesc {
    /// 资源 ID（RenderGraph 内部）
    pub resource_id: u32,
    /// 源状态
    pub src_state: ImageState,
    /// 目标状态
    pub dst_state: ImageState,
    /// 图像 aspect（COLOR / DEPTH / STENCIL）
    pub aspect: vk::ImageAspectFlags,
}

impl ImageBarrierDesc {
    /// 创建新的图像 barrier 描述
    pub fn new(resource_id: u32, src_state: ImageState, dst_state: ImageState) -> Self {
        Self {
            resource_id,
            src_state,
            dst_state,
            aspect: vk::ImageAspectFlags::COLOR,
        }
    }

    /// 设置 aspect
    pub fn with_aspect(mut self, aspect: vk::ImageAspectFlags) -> Self {
        self.aspect = aspect;
        self
    }

    /// 检查是否需要 barrier
    ///
    /// 如果 layout 相同且 access 兼容，可能不需要 barrier
    pub fn needs_barrier(&self) -> bool {
        // Layout 不同一定需要 barrier
        if self.src_state.layout != self.dst_state.layout {
            return true;
        }

        // 有写操作需要 barrier（确保可见性）
        if self.src_state.is_write() || self.dst_state.is_write() {
            return true;
        }

        // 只读到只读可以跳过 barrier
        false
    }

    /// 转换为 GfxImageBarrier
    ///
    /// 需要提供实际的 vk::Image handle
    pub fn to_gfx_barrier(&self, image: vk::Image) -> GfxImageBarrier {
        GfxImageBarrier::new()
            .image(image)
            .layout_transfer(self.src_state.layout, self.dst_state.layout)
            .src_mask(self.src_state.stage, self.src_state.src_access())
            .dst_mask(self.dst_state.stage, self.dst_state.access)
            .image_aspect_flag(self.aspect)
    }
}

/// 缓冲区 Barrier 描述
#[derive(Clone, Debug)]
pub struct BufferBarrierDesc {
    /// 资源 ID
    pub resource_id: u32,
    /// 源状态
    pub src_state: BufferState,
    /// 目标状态
    pub dst_state: BufferState,
    /// 缓冲区偏移
    pub offset: vk::DeviceSize,
    /// 缓冲区大小（WHOLE_SIZE 表示整个缓冲区）
    pub size: vk::DeviceSize,
}

impl BufferBarrierDesc {
    /// 创建新的缓冲区 barrier 描述
    pub fn new(resource_id: u32, src_state: BufferState, dst_state: BufferState) -> Self {
        Self {
            resource_id,
            src_state,
            dst_state,
            offset: 0,
            size: vk::WHOLE_SIZE,
        }
    }

    /// 检查是否需要 barrier
    pub fn needs_barrier(&self) -> bool {
        // 有写操作需要 barrier
        self.src_state.is_write() || self.dst_state.is_write()
    }

    /// 转换为 GfxBufferBarrier
    pub fn to_gfx_barrier(&self, buffer: vk::Buffer) -> GfxBufferBarrier {
        GfxBufferBarrier::new()
            .buffer(buffer, self.offset, self.size)
            .src_mask(self.src_state.stage, self.src_state.access)
            .dst_mask(self.dst_state.stage, self.dst_state.access)
    }
}

/// Pass 执行前需要的 Barrier 集合
#[derive(Clone, Debug, Default)]
pub struct PassBarriers {
    /// 图像 barriers
    pub image_barriers: Vec<ImageBarrierDesc>,
    /// 缓冲区 barriers
    pub buffer_barriers: Vec<BufferBarrierDesc>,
}

impl PassBarriers {
    /// 创建空的 barrier 集合
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加图像 barrier
    pub fn add_image_barrier(&mut self, barrier: ImageBarrierDesc) {
        if barrier.needs_barrier() {
            self.image_barriers.push(barrier);
        }
    }

    /// 添加缓冲区 barrier
    pub fn add_buffer_barrier(&mut self, barrier: BufferBarrierDesc) {
        if barrier.needs_barrier() {
            self.buffer_barriers.push(barrier);
        }
    }

    /// 检查是否有 barrier
    pub fn has_barriers(&self) -> bool {
        !self.image_barriers.is_empty() || !self.buffer_barriers.is_empty()
    }

    /// 获取图像 barrier 数量
    pub fn image_barrier_count(&self) -> usize {
        self.image_barriers.len()
    }

    /// 获取缓冲区 barrier 数量
    pub fn buffer_barrier_count(&self) -> usize {
        self.buffer_barriers.len()
    }
}

/// Barrier 计算器
///
/// 根据资源状态跟踪信息，计算每个 Pass 需要的 barriers。
pub struct BarrierCalculator;

impl BarrierCalculator {
    /// 计算单个图像资源的 barrier
    ///
    /// # 参数
    /// - `resource_id`: 资源 ID
    /// - `current_state`: 当前状态（上一个使用者留下的状态）
    /// - `required_state`: 需要的状态
    ///
    /// # 返回
    /// 如果需要 barrier，返回 `Some(ImageBarrierDesc)`
    pub fn compute_image_barrier(
        resource_id: u32,
        current_state: ImageState,
        required_state: ImageState,
    ) -> Option<ImageBarrierDesc> {
        let barrier = ImageBarrierDesc::new(resource_id, current_state, required_state);
        if barrier.needs_barrier() { Some(barrier) } else { None }
    }

    /// 计算单个缓冲区资源的 barrier
    pub fn compute_buffer_barrier(
        resource_id: u32,
        current_state: BufferState,
        required_state: BufferState,
    ) -> Option<BufferBarrierDesc> {
        let barrier = BufferBarrierDesc::new(resource_id, current_state, required_state);
        if barrier.needs_barrier() { Some(barrier) } else { None }
    }

    /// 推断图像的 aspect flags
    ///
    /// 根据 format 自动推断（简化版本）
    pub fn infer_image_aspect(format: vk::Format) -> vk::ImageAspectFlags {
        match format {
            vk::Format::D16_UNORM | vk::Format::D32_SFLOAT | vk::Format::X8_D24_UNORM_PACK32 => {
                vk::ImageAspectFlags::DEPTH
            }

            vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,

            vk::Format::D16_UNORM_S8_UINT | vk::Format::D24_UNORM_S8_UINT | vk::Format::D32_SFLOAT_S8_UINT => {
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
            }

            _ => vk::ImageAspectFlags::COLOR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_barrier_layout_change() {
        let barrier = ImageBarrierDesc::new(0, ImageState::UNDEFINED, ImageState::COLOR_ATTACHMENT_WRITE);

        assert!(barrier.needs_barrier());
    }

    #[test]
    fn test_image_barrier_read_to_read() {
        let barrier = ImageBarrierDesc::new(0, ImageState::SHADER_READ_FRAGMENT, ImageState::SHADER_READ_COMPUTE);

        // 同 layout 的只读到只读可以跳过
        // 但这里 layout 可能不同，取决于实际定义
        // 实际上 SHADER_READ_ONLY_OPTIMAL 相同，所以不需要
        assert!(!barrier.needs_barrier());
    }

    #[test]
    fn test_image_barrier_write_to_read() {
        let barrier = ImageBarrierDesc::new(0, ImageState::STORAGE_WRITE_COMPUTE, ImageState::SHADER_READ_FRAGMENT);

        assert!(barrier.needs_barrier());
    }
}
