//! UI 渲染 Pass
//!
//! 使用 RenderGraph V2 声明式定义的 ImGui 渲染 Pass。

use truvis_render_graph::render_graph_v2::{RgImageHandle, RgImageState, RgPass, RgPassBuilder, RgPassContext};

/// UI 渲染 Pass
///
/// 在最终渲染目标上叠加 ImGui UI。
pub struct UiPass {
    /// 渲染目标（读写，UI 叠加在已有内容上）
    pub render_target: RgImageHandle,
}

impl UiPass {
    pub fn new(render_target: RgImageHandle) -> Self {
        Self { render_target }
    }
}

impl RgPass for UiPass {
    fn setup(&mut self, builder: &mut RgPassBuilder) {
        // UI 在现有图像上叠加，需要读写
        builder.read_write_image(self.render_target, RgImageState::COLOR_ATTACHMENT_WRITE);
    }

    fn execute(&self, ctx: &RgPassContext<'_>) {
        let _cmd = ctx.cmd;

        // TODO: 集成 ImGui 渲染
        // 实际实现需要：
        // 1. 获取 ImGui draw data
        // 2. 设置 rendering info
        // 3. 绑定 ImGui pipeline
        // 4. 绘制 ImGui primitives
        //
        // 当前 ImGui 渲染由 Renderer 层面处理，
        // 这里仅作为 RenderGraph 集成的示例。

        log::trace!("UiPass::execute (placeholder - actual UI rendered by Renderer)");
    }
}
