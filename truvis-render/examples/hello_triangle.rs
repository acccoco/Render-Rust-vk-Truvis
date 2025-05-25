use ash::vk;
use imgui::Ui;
use model_manager::component::TruGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use model_manager::vertex::VertexLayout;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_render::app::{AppCtx, OuterApp, TruvisApp};
use truvis_render::frame_context::FrameContext;
use truvis_render::renderer::bindless::BindlessManager;
use truvis_render::renderer::gpu_scene::GpuScene;
use truvis_render::renderer::framebuffer::FrameBuffer;
use truvis_render::renderer::scene_manager::TheWorld;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::graphics_pipeline::RhiGraphicsPipelineCreateInfo;
use truvis_rhi::{
    core::{command_queue::RhiSubmitInfo, graphics_pipeline::RhiGraphicsPipeline},
    rhi::Rhi,
};

struct HelloTriangle {
    triangle: TruGeometry<VertexPosColor>,

    pipeline: RhiGraphicsPipeline,

    frame_id: usize,
}

impl HelloTriangle {
    fn init_pipeline(rhi: &Rhi, render_context: &mut FrameContext) -> RhiGraphicsPipeline {
        let mut pipeline_ci = RhiGraphicsPipelineCreateInfo::default();
        pipeline_ci.vertex_shader_stage("shader/build/hello_triangle/triangle.slang.spv", cstr::cstr!("vsmain"));
        pipeline_ci.fragment_shader_stage("shader/build/hello_triangle/triangle.slang.spv", cstr::cstr!("psmain"));
        pipeline_ci.attach_info(
            vec![render_context.color_format()],
            Some(render_context.depth_format()),
            Some(vk::Format::UNDEFINED),
        );
        pipeline_ci.vertex_binding(VertexAosLayoutPosColor::vertex_input_bindings());
        pipeline_ci.vertex_attribute(VertexAosLayoutPosColor::vertex_input_attributes());
        pipeline_ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        RhiGraphicsPipeline::new(rhi.device.clone(), &pipeline_ci, "hello-triangle-pipeline")
    }

    fn my_update(&self, rhi: &Rhi, render_context: &mut FrameContext) {
        let color_attach = FrameBuffer::get_color_attachment(render_context.current_present_image_view());
        let depth_attach = FrameBuffer::get_depth_attachment(render_context.depth_view.handle());
        let render_info = FrameBuffer::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );
        let swapchain_extend = render_context.swapchain_extent();

        let cmd = FrameContext::alloc_command_buffer(render_context, "render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[main-pass]draw");
        {
            cmd.cmd_begin_rendering(&render_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline());

            cmd.cmd_set_viewport(
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: swapchain_extend.height as f32,
                    width: swapchain_extend.width as f32,
                    height: -(swapchain_extend.height as f32),
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

            cmd.cmd_bind_index_buffer(&self.triangle.index_buffer, 0, vk::IndexType::UINT32);
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.triangle.vertex_buffer), &[0]);
            cmd.draw_indexed(self.triangle.index_cnt(), 0, 1, 0, 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn new(rhi: &Rhi, render_context: &mut FrameContext) -> Self {
        let pipeline = HelloTriangle::init_pipeline(rhi, render_context);
        let triangle = VertexAosLayoutPosColor::triangle(rhi);
        Self {
            triangle,
            pipeline,

            frame_id: 0,
        }
    }
}

impl OuterApp for HelloTriangle {
    fn init(
        rhi: &Rhi,
        render_context: &mut FrameContext,
        _scene_mgr: Rc<RefCell<TheWorld>>,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        log::info!("hello triangle init.");

        // 至少注册一个纹理，否则 bindless layout 会没有纹理绑定点
        bindless_mgr.borrow_mut().register_texture(rhi, "assets/uv_checker.png".to_string());

        HelloTriangle::new(rhi, render_context)
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
        ui.text_wrapped(format!("Frame ID: {}", self.frame_id));
        static mut UI_VALUE: usize = 0;
        let choices = ["test test this is 1", "test test this is 2"];
        unsafe {
            if ui.button(choices[UI_VALUE]) {
                UI_VALUE += 1;
                UI_VALUE %= 2;
            }
        }

        ui.button("This...is...imgui-rs!");
        ui.separator();
        let mouse_pos = ui.io().mouse_pos;
        ui.text(format!("Mouse Position: ({:.1},{:.1})", mouse_pos[0], mouse_pos[1]));
    }

    fn update(&mut self, app_ctx: &mut AppCtx) {
        self.frame_id = app_ctx.render_context.current_frame_num();
    }

    fn draw(
        &self,
        app_ctx: &mut AppCtx,
        _per_frame_data_buffer: &RhiStructuredBuffer<shader::PerFrameData>,
        _gpu_scene: &GpuScene,
    ) {
        self.my_update(app_ctx.rhi, app_ctx.render_context);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
