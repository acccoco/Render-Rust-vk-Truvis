use ash::vk;
use bytemuck::{Pod, Zeroable};
use itertools::Itertools;
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use model_manager::vertex::VertexLayout;
use std::rc::Rc;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::create_named_array;
use truvis_render::pipeline_settings::FrameSettings;
use truvis_render::platform::timer::Timer;
use truvis_render::renderer::frame_controller::FrameController;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::graphics_pipeline::{RhiGraphicsPipeline, RhiGraphicsPipelineCreateInfo, RhiPipelineLayout};
use truvis_rhi::core::rendering_info::RhiRenderingInfo;
use truvis_rhi::core::shader::RhiShaderStageInfo;
use truvis_rhi::rhi::Rhi;

create_named_array!(
    ShaderStage,
    SHADER_STAGES,
    RhiShaderStageInfo,
    [
        (
            Vertex,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                entry_point: cstr::cstr!("main"),
                path: "shader/build/shadertoy-glsl/shadertoy.vert.spv",
            }
        ),
        (
            Fragment,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                entry_point: cstr::cstr!("main"),
                path: "shader/build/shadertoy-glsl/shadertoy.frag.spv",
            }
        ),
    ]
);

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
pub struct PushConstants {
    /// 鼠标位置和状态
    mouse: glam::Vec4,
    /// 分辨率
    resolution: glam::Vec2,
    /// 播放时间 seconds
    time: f32,
    /// frame 渲染时间 seconds
    delta_time: f32,
    /// 累计渲染帧数
    frame: i32,
    /// 帧率
    frame_rate: f32,
    /// padding
    __padding__: [f32; 2],
}

pub struct ShaderToyPass {
    pipeline: RhiGraphicsPipeline,
    _pipeline_layout: Rc<RhiPipelineLayout>,
}
impl ShaderToyPass {
    pub fn new(rhi: &Rhi, color_format: vk::Format) -> Self {
        let mut pipeline_ci = RhiGraphicsPipelineCreateInfo::default();
        pipeline_ci.shader_stages(ShaderStage::iter().map(|stage| *stage.value()).collect_vec());
        pipeline_ci.attach_info(vec![color_format], None, Some(vk::Format::UNDEFINED));
        pipeline_ci.vertex_binding(VertexAosLayoutPosColor::vertex_input_bindings());
        pipeline_ci.vertex_attribute(VertexAosLayoutPosColor::vertex_input_attributes());
        pipeline_ci.color_blend(
            vec![vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)],
            [0.0; 4],
        );

        let pipeline_layout = Rc::new(RhiPipelineLayout::new(
            rhi.device.clone(),
            &[],
            &[vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<PushConstants>() as u32,
            }],
            "shader-toy",
        ));
        let pipeline =
            RhiGraphicsPipeline::new(rhi.device.clone(), &pipeline_ci, pipeline_layout.clone(), "shader-toy");

        Self {
            _pipeline_layout: pipeline_layout,
            pipeline,
        }
    }

    pub fn draw(
        &self,
        cmd: &RhiCommandBuffer,
        frame_ctrl: &FrameController,
        frame_settings: &FrameSettings,
        render_target: vk::ImageView,
        timer: &Timer,
        rect: &DrsGeometry<VertexPosColor>,
    ) {
        let viewport_extent = frame_settings.frame_extent;

        let push_constants = PushConstants {
            time: timer.total_time.as_secs_f32(),
            delta_time: timer.delta_time_s(),
            frame: frame_ctrl.frame_id() as i32,
            frame_rate: 1.0 / timer.delta_time_s(),
            resolution: glam::Vec2::new(viewport_extent.width as f32, viewport_extent.height as f32),
            mouse: glam::Vec4::new(
                0.2 * (viewport_extent.width as f32),
                0.2 * (viewport_extent.height as f32),
                0.0,
                0.0,
            ),
            __padding__: [0.0, 0.0],
        };

        let rendering_info = RhiRenderingInfo::new(
            vec![render_target],
            None,
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: viewport_extent,
            },
        );

        {
            cmd.cmd_push_constants(
                self.pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&push_constants),
            );

            cmd.cmd_begin_rendering2(&rendering_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());

            cmd.cmd_set_viewport(
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: viewport_extent.width as f32,
                    height: viewport_extent.height as f32,
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

            cmd.cmd_bind_index_buffer(&rect.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&rect.vertex_buffer), &[0]);
            cmd.draw_indexed(rect.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
    }
}
