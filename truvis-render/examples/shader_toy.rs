use ash::vk;
use bytemuck::{Pod, Zeroable};
use imgui::Ui;
use model_manager::component::mesh::SimpleMesh;
use model_manager::vertex::vertex_pc::VertexAosLayoutPosColor;
use model_manager::vertex::VertexLayout;
use truvis_render::render::{App, AppCtx, AppInitInfo, Renderer, Timer};
use truvis_render::render_context::RenderContext;
use truvis_rhi::core::pipeline::RhiGraphicsPipelineCreateInfo;
use truvis_rhi::{
    core::{command_queue::RhiSubmitInfo, pipeline::RhiGraphicsPipeline},
    rhi::Rhi,
};

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
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
    rectangle: SimpleMesh,
    pipeline: RhiGraphicsPipeline,
}

impl ShaderToy {
    fn init_pipeline(rhi: &Rhi, render_context: &RenderContext) -> RhiGraphicsPipeline {
        let extent = render_context.swapchain_extent();
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.push_constant_ranges(vec![vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::ALL,
            offset: 0,
            size: size_of::<PushConstants>() as u32,
        }]);
        ci.vertex_shader_stage("shader/shadertoy-glsl/shadertoy.vert.spv".to_string(), "main".to_string());
        ci.fragment_shader_stage("shader/shadertoy-glsl/shadertoy.frag.spv".to_string(), "main".to_string());
        ci.attach_info(vec![render_context.color_format()], Some(render_context.depth_format()), None);
        ci.viewport(glam::vec2(0.0, 0.0), glam::vec2(extent.width as f32, extent.height as f32), 0.0, 1.0);
        ci.scissor(extent.into());
        ci.vertex_binding(VertexAosLayoutPosColor::vertex_input_bindings());
        ci.vertex_attribute(VertexAosLayoutPosColor::vertex_input_attributes());
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "shadertoy")
    }

    fn run(&self, rhi: &Rhi, render_context: &mut RenderContext, timer: &Timer) {
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

        let depth_attach_info = <Self as App>::get_depth_attachment(render_context.depth_view.handle());
        let color_attach_info = <Self as App>::get_color_attachment(render_context.current_present_image_view());
        let render_info = <Self as App>::get_render_info(
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
                self.pipeline.pipeline_layout,
                vk::ShaderStageFlags::ALL,
                0,
                bytemuck::bytes_of(&push_constants),
            );

            cmd.cmd_begin_rendering(&render_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
            cmd.cmd_bind_index_buffer(&self.rectangle.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.rectangle.vertex_buffer), &[0]);
            cmd.draw_indexed(self.rectangle.index_cnt, 0, 1, 0, 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn new(rhi: &Rhi, render_context: &mut RenderContext) -> Self {
        log::info!("start.");

        let pipeline = Self::init_pipeline(rhi, render_context);
        let rectangle = VertexAosLayoutPosColor::rectangle(rhi);

        Self { rectangle, pipeline }
    }
}

impl App for ShaderToy {
    fn update_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, _app_ctx: &mut AppCtx) {
        //
    }

    fn draw(&self, app_ctx: &mut AppCtx) {
        self.run(app_ctx.rhi, app_ctx.render_context, app_ctx.timer)
    }

    fn init(rhi: &Rhi, render_context: &mut RenderContext) -> Self {
        ShaderToy::new(rhi, render_context)
    }

    fn get_render_init_info() -> AppInitInfo {
        AppInitInfo {
            window_width: 1600,
            window_height: 900,
            app_name: "hello-triangle".to_string(),
            enable_validation: true,
        }
    }
}

fn main() {
    Renderer::<ShaderToy>::run();
}
