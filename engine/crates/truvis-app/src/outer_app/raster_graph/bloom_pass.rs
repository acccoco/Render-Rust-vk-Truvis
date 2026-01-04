//! Bloom 后处理 Pass
//!
//! 使用 RenderGraph V2 声明式定义的 Bloom 效果。
//! 简化版本：仅演示 RenderGraph 的使用方式。

use truvis_render_graph::render_graph_v2::{RgImageHandle, RgImageState, RgPass, RgPassBuilder, RgPassContext};

/// Bloom 后处理 Pass
///
/// 读取场景渲染结果，应用 Bloom 效果后写入输出。
///
/// 注意：这是一个简化的占位实现，实际 Bloom 需要：
/// 1. 提取高亮区域
/// 2. 多次降采样模糊
/// 3. 多次升采样混合
/// 4. 与原图合成
pub struct BloomPass {
    /// 输入图像（场景渲染结果）
    pub input: RgImageHandle,
    /// 输出图像（Bloom 后的结果）
    pub output: RgImageHandle,

    /// 是否启用 Bloom
    pub enabled: bool,
}

impl BloomPass {
    pub fn new(input: RgImageHandle, output: RgImageHandle, enabled: bool) -> Self {
        Self { input, output, enabled }
    }
}

impl RgPass for BloomPass {
    fn setup(&mut self, builder: &mut RgPassBuilder) {
        if self.enabled {
            // 读取输入图像
            builder.read_image(self.input, RgImageState::SHADER_READ_COMPUTE);
            // 写入输出图像
            builder.write_image(self.output, RgImageState::STORAGE_WRITE_COMPUTE);
        }
    }

    fn execute(&self, ctx: &RgPassContext<'_>) {
        if !self.enabled {
            return;
        }

        let _cmd = ctx.cmd;

        // TODO: 实现实际的 Bloom compute shader
        // 1. 绑定 input 作为 sampled image
        // 2. 绑定 output 作为 storage image
        // 3. Dispatch compute shader
        //
        // 当前为占位实现，直接 copy input -> output
        // 实际实现需要：
        // - Threshold pass: 提取高亮
        // - Downsample passes: 逐级降采样
        // - Upsample passes: 逐级升采样并混合
        // - Composite pass: 与原图混合

        log::trace!("BloomPass::execute (placeholder)");
    }
}
