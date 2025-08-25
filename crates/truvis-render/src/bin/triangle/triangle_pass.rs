use ash::vk;
use itertools::Itertools;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use model_manager::vertex::VertexLayout;
use std::rc::Rc;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::const_map;
use truvis_crate_tools::resource::TruvisPath;
use truvis_render::pipeline_settings::{FrameLabel, FrameSettings};
use truvis_render::renderer::frame_buffers::FrameBuffers;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::graphics_pipeline::{RhiGraphicsPipeline, RhiGraphicsPipelineCreateInfo, RhiPipelineLayout};
use truvis_rhi::core::rendering_info::RhiRenderingInfo;
use truvis_rhi::core::shader::RhiShaderStageInfo;
use truvis_rhi::rhi::Rhi;

const_map!(ShaderStage<RhiShaderStageInfo>: {
    Vertex: RhiShaderStageInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        entry_point: cstr::cstr!("vsmain"),
        path: TruvisPath::shader_path("hello_triangle/triangle.slang.spv"),
    },
    Fragment: RhiShaderStageInfo {
        stage: vk::ShaderStageFlags::FRAGMENT,
        entry_point: cstr::cstr!("psmain"),
        path: TruvisPath::shader_path("hello_triangle/triangle.slang.spv"),
    },
});

pub struct TrianglePass {
    pipeline: RhiGraphicsPipeline,
    _pipeline_layout: Rc<RhiPipelineLayout>,
}
impl TrianglePass {
    pub fn new(rhi: &Rhi, frame_settings: &FrameSettings) -> Self {
        let mut pipeline_ci = RhiGraphicsPipelineCreateInfo::default();
        pipeline_ci.shader_stages(ShaderStage::iter().map(|stage| stage.value().clone()).collect_vec());
        pipeline_ci.attach_info(vec![frame_settings.color_format], None, Some(vk::Format::UNDEFINED));
        pipeline_ci.vertex_binding(VertexAosLayoutPosColor::vertex_input_bindings());
        pipeline_ci.vertex_attribute(VertexAosLayoutPosColor::vertex_input_attributes());
        pipeline_ci.color_blend(
            vec![vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)],
            [0.0; 4],
        );

        let pipeline_layout = Rc::new(RhiPipelineLayout::new(rhi.device.clone(), &[], &[], "hello-triangle"));
        let pipeline = RhiGraphicsPipeline::new(
            rhi.device.clone(),
            &pipeline_ci,
            pipeline_layout.clone(),
            "hello-triangle-pipeline",
        );

        Self {
            _pipeline_layout: pipeline_layout,
            pipeline,
        }
    }

    pub fn draw(
        &self,
        cmd: &RhiCommandBuffer,
        frame_label: FrameLabel,
        framebuffers: &FrameBuffers,
        frame_settings: &FrameSettings,
        shape: &DrsGeometry<VertexPosColor>,
    ) {
        let viewport_extent = frame_settings.frame_extent;
        let rendering_info = RhiRenderingInfo::new(
            vec![framebuffers.render_target_image_view(frame_label).handle()],
            None,
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: viewport_extent,
            },
        );

        {
            cmd.cmd_begin_rendering2(&rendering_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());

            cmd.cmd_set_viewport(
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: viewport_extent.height as f32,
                    width: viewport_extent.width as f32,
                    height: -(viewport_extent.height as f32),
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            cmd.cmd_set_scissor(
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: viewport_extent,
                }],
            );

            cmd.cmd_bind_index_buffer(&shape.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&shape.vertex_buffer), &[0]);
            cmd.draw_indexed(shape.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
    }
}
