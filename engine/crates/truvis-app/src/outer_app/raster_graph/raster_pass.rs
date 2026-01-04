//! 光栅化渲染 Pass
//!
//! 使用 RenderGraph V2 声明式定义的场景渲染 Pass。

use std::rc::Rc;

use ash::vk;
use itertools::Itertools;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::enumed_map;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::pipelines::graphics_pipeline::{GfxGraphicsPipeline, GfxGraphicsPipelineCreateInfo, GfxPipelineLayout};
use truvis_gfx::pipelines::rendering_info::GfxRenderingInfo;
use truvis_gfx::pipelines::shader::GfxShaderStageInfo;
use truvis_gfx::resources::layout::GfxVertexLayout;
use truvis_gfx::resources::vertex_layout::soa_3d::VertexLayoutSoA3D;
use truvis_render_graph::render_graph_v2::{RgImageHandle, RgImageState, RgPass, RgPassBuilder, RgPassContext};
use truvis_render_interface::geometry::RtGeometry;
use truvis_render_interface::pipeline_settings::FrameSettings;

enumed_map!(RasterShaderStage<GfxShaderStageInfo>: {
    Vertex: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        entry_point: c"vsmain",
        path: TruvisPath::shader_build_path_str("hello_triangle/triangle.slang"),
    },
    Fragment: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::FRAGMENT,
        entry_point: c"psmain",
        path: TruvisPath::shader_build_path_str("hello_triangle/triangle.slang"),
    },
});

/// 光栅化管线资源（由 App 持有，跨帧复用）
pub struct RasterPipeline {
    pub pipeline: GfxGraphicsPipeline,
    pub pipeline_layout: Rc<GfxPipelineLayout>,
}

impl RasterPipeline {
    pub fn new(frame_settings: &FrameSettings) -> Self {
        let mut pipeline_ci = GfxGraphicsPipelineCreateInfo::default();
        pipeline_ci.shader_stages(RasterShaderStage::iter().map(|s| s.value().clone()).collect_vec());
        pipeline_ci.attach_info(
            vec![frame_settings.color_format],
            Some(frame_settings.depth_format),
            Some(vk::Format::UNDEFINED),
        );
        pipeline_ci.vertex_binding(VertexLayoutSoA3D::vertex_input_bindings());
        pipeline_ci.vertex_attribute(VertexLayoutSoA3D::vertex_input_attributes());
        pipeline_ci.depth_test(Some(vk::CompareOp::LESS), true, false);
        pipeline_ci.color_blend(
            vec![
                vk::PipelineColorBlendAttachmentState::default()
                    .blend_enable(false)
                    .color_write_mask(vk::ColorComponentFlags::RGBA),
            ],
            [0.0; 4],
        );

        let pipeline_layout = Rc::new(GfxPipelineLayout::new(&[], &[], "raster-graph-pipeline-layout"));
        let pipeline = GfxGraphicsPipeline::new(&pipeline_ci, pipeline_layout.clone(), "raster-graph-pipeline");

        Self {
            pipeline,
            pipeline_layout,
        }
    }
}

/// 光栅化渲染 Pass
///
/// 直接借用外部资源（pipeline、geometry），避免不必要的引用计数。
/// 通过 RenderGraphBuilder 的生命周期参数约束借用的有效性。
///
/// 注意：resource_manager 通过 PassContext 在 execute 时访问。
pub struct RasterPass<'a> {
    /// 渲染目标句柄（RenderGraph 虚拟句柄）
    pub render_target: RgImageHandle,
    /// 深度缓冲句柄
    pub depth_target: RgImageHandle,

    /// 借用管线
    pub pipeline: &'a GfxGraphicsPipeline,
    /// 借用几何体
    pub geometry: &'a RtGeometry,

    /// Frame extent
    pub frame_extent: vk::Extent2D,
}

impl<'a> RasterPass<'a> {
    /// 创建 Pass 实例
    pub fn new(
        render_target: RgImageHandle,
        depth_target: RgImageHandle,
        pipeline: &'a RasterPipeline,
        geometry: &'a RtGeometry,
        frame_extent: vk::Extent2D,
    ) -> Self {
        Self {
            render_target,
            depth_target,
            pipeline: &pipeline.pipeline,
            geometry,
            frame_extent,
        }
    }
}

impl RgPass for RasterPass<'_> {
    fn setup(&mut self, builder: &mut RgPassBuilder) {
        // 声明写入 render target
        builder.write_image(self.render_target, RgImageState::COLOR_ATTACHMENT_WRITE);
        // 声明写入 depth buffer
        builder.write_image(self.depth_target, RgImageState::DEPTH_ATTACHMENT_WRITE);
    }

    fn execute(&self, ctx: &RgPassContext<'_>) {
        let cmd = ctx.cmd;

        // 获取物理 image view handle
        let (_, render_target_view_handle) =
            ctx.get_image(self.render_target).expect("RasterPass: render_target not found");
        let (_, depth_view_handle) = ctx.get_image(self.depth_target).expect("RasterPass: depth not found");

        // 从 PassContext 的 resource_manager 获取实际的 view
        let render_target_view = ctx.resource_manager.get_image_view(render_target_view_handle).unwrap();
        let depth_view = ctx.resource_manager.get_image_view(depth_view_handle).unwrap();

        let rendering_info = GfxRenderingInfo::new(
            vec![render_target_view.handle()],
            Some(depth_view.handle()),
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: self.frame_extent,
            },
        );

        cmd.cmd_begin_rendering2(&rendering_info);
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());

        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: 0.0,
                y: self.frame_extent.height as f32,
                width: self.frame_extent.width as f32,
                height: -(self.frame_extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: self.frame_extent,
            }],
        );

        self.geometry.cmd_bind_index_buffer(cmd);
        self.geometry.cmd_bind_vertex_buffers(cmd);
        cmd.draw_indexed(self.geometry.index_cnt(), 0, 1, 0, 0);

        cmd.end_rendering();
    }
}
