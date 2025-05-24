use ash::vk;
use bytemuck::{Pod, Zeroable};
use imgui::Ui;
use model_manager::component::TruGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use model_manager::vertex::VertexLayout;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_render::app::{AppCtx, OuterApp, TruvisApp};
use truvis_render::frame_context::FrameContext;
use truvis_render::platform::camera_controller::CameraController;
use truvis_render::platform::timer::Timer;
use truvis_render::renderer::framebuffer::FrameBuffer;
use truvis_rhi::core::graphics_pipeline::RhiGraphicsPipelineCreateInfo;
use truvis_rhi::{
    core::{command_queue::RhiSubmitInfo, graphics_pipeline::RhiGraphicsPipeline},
    rhi::Rhi,
};

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

struct ShaderToy {
    rectangle: TruGeometry<VertexPosColor>,
    pipeline: RhiGraphicsPipeline,
}

impl ShaderToy {
    fn init_pipeline(rhi: &Rhi, render_context: &FrameContext) -> RhiGraphicsPipeline {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.push_constant_ranges(vec![vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: size_of::<PushConstants>() as u32,
        }]);
        ci.vertex_shader_stage("shader/build/shadertoy-glsl/shadertoy.vert.spv", cstr::cstr!("main"));
        ci.fragment_shader_stage("shader/build/shadertoy-glsl/shadertoy.frag.spv", cstr::cstr!("main"));
        ci.attach_info(vec![render_context.color_format()], Some(render_context.depth_format()), None);
        ci.vertex_binding(VertexAosLayoutPosColor::vertex_input_bindings());
        ci.vertex_attribute(VertexAosLayoutPosColor::vertex_input_attributes());
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "shadertoy")
    }

    fn run(&self, rhi: &Rhi, render_context: &mut FrameContext, timer: &Timer) {
        let push_constants = PushConstants {
            time: timer.total_time_s,
            delta_time: timer.delta_time_s,
            frame: timer.total_frame,
            frame_rate: 1.0 / timer.delta_time_s,
            resolution: glam::Vec2::new(
                render_context.swapchain_extent().width as f32,
                render_context.swapchain_extent().height as f32,
            ),
            mouse: glam::Vec4::new(
                0.2 * (render_context.swapchain_extent().width as f32),
                0.2 * (render_context.swapchain_extent().height as f32),
                0.0,
                0.0,
            ),
            __padding__: [0.0, 0.0],
        };

        let depth_attach_info = FrameBuffer::get_depth_attachment(render_context.depth_view.handle());
        let color_attach_info = FrameBuffer::get_color_attachment(render_context.current_present_image_view());
        let render_info = FrameBuffer::get_render_info(
            vk::Rect2D {
                offset: Default::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach_info),
            &depth_attach_info,
        );

        let cmd = render_context.alloc_command_buffer("render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[main-pass]draw");
        {
            cmd.cmd_push_constants(
                self.pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&push_constants),
            );

            cmd.cmd_begin_rendering(&render_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline());

            let swapchain_extend = render_context.swapchain_extent();
            cmd.cmd_set_viewport(
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: swapchain_extend.width as f32,
                    height: swapchain_extend.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            cmd.cmd_set_scissor(
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D::default(),
                    extent: swapchain_extend,
                }],
            );

            cmd.cmd_bind_index_buffer(&self.rectangle.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.rectangle.vertex_buffer), &[0]);
            cmd.draw_indexed(self.rectangle.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn new(rhi: &Rhi, render_context: &mut FrameContext) -> Self {
        log::info!("start.");

        let pipeline = Self::init_pipeline(rhi, render_context);
        let rectangle = VertexAosLayoutPosColor::rectangle(rhi);

        Self { rectangle, pipeline }
    }
}

impl OuterApp for ShaderToy {
    fn init(rhi: &Rhi, render_context: &mut FrameContext, _camera_controller: Rc<RefCell<CameraController>>) -> Self {
        ShaderToy::new(rhi, render_context)
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, _app_ctx: &mut AppCtx) {
        //
    }

    fn draw(&self, app_ctx: &mut AppCtx) {
        self.run(app_ctx.rhi, app_ctx.render_context, app_ctx.timer)
    }
}

fn main() {
    TruvisApp::<ShaderToy>::run();
}
