use crate::platform::camera::DrsCamera;
use crate::platform::input_manager::InputState;
use crate::platform::timer::Timer;
use crate::render_pipeline::pipeline_context::TempPipelineCtx;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::FrameContext;
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::pipeline_settings::{
    AccumData, DefaultRendererSettings, FifLabel, PipelineSettings, RendererSettings,
};
use crate::renderer::scene_manager::SceneManager;
use crate::renderer::window_system::MainWindow;
use ash::vk;
use shader_binding::shader;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::synchronize::{RhiBarrierMask, RhiBufferBarrier};
use truvis_rhi::{core::command_queue::RhiSubmitInfo, rhi::Rhi};

/// 表示整个渲染器进程，需要考虑 platform, render, rhi, log 之类的各种模块
pub struct Renderer {
    pub rhi: Rc<Rhi>,

    /// 需要在 window 存在后创建，且需要手动释放和重新创建，因此使用 Option
    pub frame_ctx: Option<FrameContext>,

    pipeline_settings: PipelineSettings,

    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub scene_mgr: Rc<RefCell<SceneManager>>,
    pub gpu_scene: GpuScene,

    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,
    accum_data: AccumData,
}
impl Drop for Renderer {
    fn drop(&mut self) {
        log::info!("Dropping Renderer");
        // 在 Renderer 被销毁时，等待 Rhi 设备空闲
        self.wait_idle();

        if let Some(render_context) = self.frame_ctx.take() {
            render_context.destroy(&mut self.bindless_mgr.borrow_mut());
        }
    }
}
// region getter
impl Renderer {
    #[inline]
    pub fn renderer_settings(&self) -> RendererSettings {
        RendererSettings {
            pipeline_settings: self.pipeline_settings,
            frame_settings: self.frame_ctx.as_ref().unwrap().frame_settings(),
        }
    }

    #[inline]
    pub fn crt_frame_label(&self) -> FifLabel {
        self.frame_ctx.as_ref().unwrap().crt_frame_label()
    }

    #[inline]
    pub fn frame_context_mut(&mut self) -> &mut FrameContext {
        self.frame_ctx.as_mut().unwrap()
    }

    #[inline]
    pub fn frame_context(&self) -> &FrameContext {
        self.frame_ctx.as_ref().unwrap()
    }

    /// 根据 vulkan 实例和显卡，获取合适的深度格式
    fn get_depth_format(rhi: &Rhi) -> vk::Format {
        rhi.find_supported_format(
            DefaultRendererSettings::DEPTH_FORMAT_CANDIDATES,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
        .first()
        .copied()
        .unwrap_or(vk::Format::UNDEFINED)
    }
}
// endregion
// region init
impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        let rhi = Rc::new(Rhi::new("Truvis".to_string(), extra_instance_ext));

        let bindless_mgr = Rc::new(RefCell::new(BindlessManager::new(&rhi, FifLabel::FRAMES_IN_FLIGHT)));
        let scene_mgr = Rc::new(RefCell::new(SceneManager::new(bindless_mgr.clone())));
        let gpu_scene = GpuScene::new(&rhi, scene_mgr.clone(), bindless_mgr.clone(), FifLabel::FRAMES_IN_FLIGHT);
        let per_frame_data_buffers = (0..FifLabel::FRAMES_IN_FLIGHT)
            .map(|idx| {
                RhiStructuredBuffer::<shader::PerFrameData>::new_ubo(&rhi, 1, format!("per-frame-data-buffer-{idx}"))
            })
            .collect();

        let pipeline_settings = PipelineSettings {
            color_format: DefaultRendererSettings::DEFAULT_SURFACE_FORMAT.format,
            depth_format: Self::get_depth_format(&rhi),
            frames_in_flight: FifLabel::FRAMES_IN_FLIGHT,
        };

        Self {
            pipeline_settings,
            accum_data: Default::default(),
            frame_ctx: None,
            rhi,
            bindless_mgr,
            scene_mgr,
            gpu_scene,
            per_frame_data_buffers,
        }
    }

    /// 在 window 创建之后调用，初始化其他资源
    pub fn init_after_window(&mut self, window: &MainWindow) {
        self.rebuild_after_resized(window);
    }
}
// endregion
// region phase call
impl Renderer {
    pub fn begin_frame(&mut self) {
        self.frame_ctx.as_mut().unwrap().begin_frame();
    }

    pub fn end_frame(&mut self) {
        self.frame_ctx.as_mut().unwrap().end_frame(&self.rhi);
    }

    pub fn before_render(&mut self, input_state: &InputState, timer: &Timer, camera: &DrsCamera) {
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);
        self.accum_data.update_accum_frames(current_camera_dir, camera.position);
        self.update_gpu_scene(input_state, timer, camera);
    }

    pub fn after_render(&mut self) {
        self.frame_ctx.as_mut().unwrap().after_render();
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }
    }

    pub fn collect_render_ctx(&mut self) -> TempPipelineCtx {
        let crt_frame_label = self.crt_frame_label();

        TempPipelineCtx {
            rhi: Some(&self.rhi),
            gpu_scene: Some(&self.gpu_scene),
            bindless_mgr: Some(self.bindless_mgr.clone()),
            per_frame_data: Some(&self.per_frame_data_buffers[*crt_frame_label]),
            frame_ctx: Some(self.frame_ctx.as_mut().unwrap()),

            gui: None,
            timer: None,
        }
    }

    /// 在窗口大小改变是，重建 swapchain
    pub fn rebuild_after_resized(&mut self, window: &MainWindow) {
        // 确保 swapchain 已经 drop 掉之后，再创建新的 swapchian，
        // 因为同一时间只能有一个 swapchain 在使用 window
        if let Some(render_context) = self.frame_ctx.take() {
            render_context.destroy(&mut self.bindless_mgr.borrow_mut());
        }

        self.accum_data.reset();
        self.frame_ctx =
            Some(FrameContext::new(&self.rhi, window, &self.pipeline_settings, &mut self.bindless_mgr.borrow_mut()));
    }

    pub fn on_render_area_changed(&mut self, region: vk::Rect2D) {
        // 只有 frame_context 中的渲染区域发生变化时，才需要重新构建
        if self.frame_context().frame_settings().rt_extent != region.extent {
            self.wait_idle();
        }

        self.frame_ctx.as_mut().unwrap().on_render_area_changed(
            &self.rhi,
            &self.pipeline_settings,
            region,
            &mut self.bindless_mgr.borrow_mut(),
        );
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, timer: &Timer, camera: &DrsCamera) {
        let frame_ctx = self.frame_ctx.as_mut().unwrap();
        let viewport_extent = frame_ctx.frame_settings().viewport_extent;

        let crt_frame_label = frame_ctx.crt_frame_label();

        // 将数据上传到 gpu buffer 中
        let cmd = frame_ctx.alloc_command_buffer("update-draw-buffer");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[update-draw-buffer]stage-to-ubo");

        let transfer_barrier_mask = RhiBarrierMask {
            src_stage: vk::PipelineStageFlags2::TRANSFER,
            src_access: vk::AccessFlags2::TRANSFER_WRITE,
            dst_stage: vk::PipelineStageFlags2::VERTEX_SHADER
                | vk::PipelineStageFlags2::FRAGMENT_SHADER
                | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            dst_access: vk::AccessFlags2::SHADER_READ,
        };

        self.gpu_scene.prepare_render_data(crt_frame_label);
        self.gpu_scene.upload_to_buffer(&self.rhi, crt_frame_label, &cmd, transfer_barrier_mask);

        // 准备好当前帧的数据
        let per_frame_data = {
            let mouse_pos = input_state.crt_mouse_pos;

            let view = camera.get_view_matrix();
            let projection = camera.get_projection_matrix();

            shader::PerFrameData {
                projection: projection.into(),
                view: view.into(),
                inv_view: view.inverse().into(),
                inv_projection: projection.inverse().into(),
                camera_pos: camera.position.into(),
                camera_forward: camera.camera_forward().into(),
                time_ms: timer.duration.as_millis() as f32,
                delta_time_ms: timer.delta_time_s * 1000.0,
                frame_id: frame_ctx.crt_frame_id() as u64,
                mouse_pos: shader::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: shader::Float2 {
                    x: viewport_extent.width as f32,
                    y: viewport_extent.height as f32,
                },
                rt_render_target: frame_ctx.crt_rt_bindless_handle(&self.bindless_mgr.borrow()),
                accum_frames: self.accum_data.accum_frames_num as u32,
            }
        };
        cmd.cmd_update_buffer(
            self.per_frame_data_buffers[*crt_frame_label].handle(),
            0,
            bytemuck::bytes_of(&per_frame_data),
        );
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiBufferBarrier::default()
                .buffer(self.per_frame_data_buffers[*crt_frame_label].handle(), 0, vk::WHOLE_SIZE)
                .mask(transfer_barrier_mask)],
        );
        cmd.end();
        self.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }
}
// endregion
