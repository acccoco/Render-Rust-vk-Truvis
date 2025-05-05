use ash::vk;
use imgui::Ui;
use model_manager::component::mesh::SimpleMesh;
use model_manager::vertex::vertex_pc::VertexAosLayoutPosColor;
use model_manager::vertex::VertexLayout;
use truvis_render::render::{App, AppCtx, AppInitInfo, Renderer};
use truvis_render::render_context::RenderContext;
use truvis_rhi::core::pipeline::RhiGraphicsPipelineCreateInfo;
use truvis_rhi::{
    core::{command_queue::RhiSubmitInfo, pipeline::RhiGraphicsPipeline},
    rhi::Rhi,
};

struct HelloTriangle {
    triangle: SimpleMesh,

    pipeline: RhiGraphicsPipeline,

    frame_id: u64,
}

impl HelloTriangle {
    fn init_pipeline(rhi: &Rhi, render_context: &mut RenderContext) -> RhiGraphicsPipeline {
        let extent = render_context.swapchain_extent();
        let mut pipeline_ci = RhiGraphicsPipelineCreateInfo::default();
        pipeline_ci
            .vertex_shader_stage("shader/build/hello_triangle/triangle.slang.spv".to_string(), "vsmain".to_string());
        pipeline_ci
            .fragment_shader_stage("shader/build/hello_triangle/triangle.slang.spv".to_string(), "psmain".to_string());
        pipeline_ci.attach_info(
            vec![render_context.color_format()],
            Some(render_context.depth_format()),
            Some(vk::Format::UNDEFINED),
        );
        pipeline_ci.viewport(
            glam::vec2(0.0, extent.height as f32),
            glam::vec2(extent.width as f32, -(extent.height as f32)),
            0.0,
            1.0,
        );
        pipeline_ci.scissor(extent.into());
        pipeline_ci.vertex_binding(VertexAosLayoutPosColor::vertex_input_bindings());
        pipeline_ci.vertex_attribute(VertexAosLayoutPosColor::vertex_input_attributes());
        pipeline_ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        RhiGraphicsPipeline::new(rhi.device.clone(), &pipeline_ci, "hello-triangle-pipeline")
    }

    fn my_update(&self, rhi: &Rhi, render_context: &mut RenderContext) {
        let color_attach = <Self as App>::get_color_attachment(render_context.current_present_image_view());
        let depth_attach = <Self as App>::get_depth_attachment(render_context.depth_view.handle());
        let render_info = <Self as App>::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );

        let cmd = RenderContext::alloc_command_buffer(render_context, "render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[main-pass]draw");
        {
            cmd.cmd_begin_rendering(&render_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
            cmd.cmd_bind_index_buffer(&self.triangle.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.triangle.vertex_buffer), &[0]);
            cmd.draw_indexed(self.triangle.index_cnt, 0, 1, 0, 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn new(rhi: &Rhi, render_context: &mut RenderContext) -> Self {
        let pipeline = HelloTriangle::init_pipeline(rhi, render_context);
        let triangle = VertexAosLayoutPosColor::triangle(rhi);
        Self {
            triangle,
            pipeline,

            frame_id: 0,
        }
    }
}

impl App for HelloTriangle {
    fn update_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
        ui.text_wrapped(format!("Frame ID: {}", self.frame_id));
        let choices = ["test test this is 1", "test test this is 2"];
        let mut value = 0;
        if ui.button(choices[value]) {
            value += 1;
            value %= 2;
        }

        ui.button("This...is...imgui-rs!");
        ui.separator();
        let mouse_pos = ui.io().mouse_pos;
        ui.text(format!("Mouse Position: ({:.1},{:.1})", mouse_pos[0], mouse_pos[1]));
    }

    fn update(&mut self, app_ctx: &mut AppCtx) {
        self.frame_id = app_ctx.render_context.frame_id;
    }

    fn draw(&self, app_ctx: &mut AppCtx) {
        self.my_update(app_ctx.rhi, app_ctx.render_context);
    }

    fn init(rhi: &Rhi, render_context: &mut RenderContext) -> Self {
        log::info!("start.");
        HelloTriangle::new(rhi, render_context)
    }

    fn get_render_init_info() -> AppInitInfo {
        AppInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "hello-triangle".to_string(),
            enable_validation: true,
        }
    }
}

fn main() {
    Renderer::<HelloTriangle>::run();
}
