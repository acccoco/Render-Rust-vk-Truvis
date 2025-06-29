use crate::pipeline_settings::{AccumData, DefaultRendererSettings, FrameLabel, FrameSettings};
use crate::platform::camera::DrsCamera;
use crate::platform::input_manager::InputState;
use crate::platform::timer::Timer;
use crate::render_pipeline::pipeline_context::TempPipelineCtx;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::{FrameContext, RendererData};
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::scene_manager::SceneManager;
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

    pub frame_ctx: FrameContext,

    frame_settings: FrameSettings,

    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub scene_mgr: Rc<RefCell<SceneManager>>,
    pub gpu_scene: GpuScene,

    // TODO 优化一下这个 buffer，不该放在这里
    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,
    accum_data: AccumData,

    timer: Timer,
    fps_limit: f32,
}
impl Drop for Renderer {
    fn drop(&mut self) {
        log::info!("Dropping Renderer");
        // 在 Renderer 被销毁时，等待 Rhi 设备空闲
        self.wait_idle();
    }
}
impl Renderer {
    // region getter

    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
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

    pub fn get_renderer_data(&self) -> Option<RendererData> {
        self.frame_ctx.get_renderer_data()
    }

    // endregion

    // region ===================================== init =====================================

    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        let rhi = Rc::new(Rhi::new("Truvis".to_string(), extra_instance_ext));

        let bindless_mgr = Rc::new(RefCell::new(BindlessManager::new(&rhi, FrameLabel::FRAMES_IN_FLIGHT)));
        let scene_mgr = Rc::new(RefCell::new(SceneManager::new(bindless_mgr.clone())));
        let gpu_scene = GpuScene::new(&rhi, scene_mgr.clone(), bindless_mgr.clone(), FrameLabel::FRAMES_IN_FLIGHT);
        let per_frame_data_buffers = (0..FrameLabel::FRAMES_IN_FLIGHT)
            .map(|idx| {
                RhiStructuredBuffer::<shader::PerFrameData>::new_ubo(&rhi, 1, format!("per-frame-data-buffer-{idx}"))
            })
            .collect();

        let frame_settings = FrameSettings {
            fif_num: FrameLabel::FRAMES_IN_FLIGHT,
            color_format: DefaultRendererSettings::DEFAULT_SURFACE_FORMAT.format,
            depth_format: Self::get_depth_format(&rhi),
            frame_extent: vk::Extent2D {
                width: 400,
                height: 400,
            },
        };
        let frame_ctx = FrameContext::new(&rhi, &frame_settings, &mut bindless_mgr.borrow_mut());

        Self {
            frame_settings,
            accum_data: Default::default(),
            frame_ctx,
            rhi,
            bindless_mgr,
            scene_mgr,
            gpu_scene,
            per_frame_data_buffers,
            timer: Timer::default(),
            fps_limit: 59.9,
        }
    }

    // endregion ====================================================================

    // region ================================== phase call ====================================

    pub fn begin_frame(&mut self) {
        self.frame_ctx.begin_frame();
        self.timer.tic();
    }

    pub fn end_frame(&mut self) {
        self.frame_ctx.end_frame(&self.rhi);
    }

    pub fn time_to_render(&self) -> bool {
        // 时间未到时，直接返回 false
        let limit_elapsed_us = 1000.0 * 1000.0 / self.fps_limit;
        if limit_elapsed_us > self.timer.toc().as_micros() as f32 {
            return false;
        }

        self.frame_ctx.time_to_render()
    }

    pub fn before_render(&mut self, input_state: &InputState, camera: &DrsCamera) {
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);
        self.accum_data.update_accum_frames(current_camera_dir, camera.position);
        self.update_gpu_scene(input_state, camera);
    }

    pub fn after_render(&mut self) {
        self.frame_ctx.after_render();
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }
    }

    pub fn collect_render_ctx(&mut self) -> TempPipelineCtx {
        let crt_frame_label = self.frame_ctx.crt_frame_label();

        TempPipelineCtx {
            rhi: Some(&self.rhi),
            gpu_scene: Some(&self.gpu_scene),
            bindless_mgr: Some(self.bindless_mgr.clone()),
            per_frame_data: Some(&self.per_frame_data_buffers[*crt_frame_label]),
            frame_ctx: Some(&mut self.frame_ctx),

            gui: None,
            timer: None,
        }
    }

    pub fn resize_frame_buffer(&mut self, new_extent: vk::Extent2D) {
        self.frame_settings.frame_extent = new_extent;
        self.frame_ctx.rebuild_framebuffers(&self.rhi, &self.frame_settings, &mut self.bindless_mgr.borrow_mut());
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, camera: &DrsCamera) {
        let frame_extent = self.frame_settings.frame_extent;
        let crt_frame_label = self.frame_ctx.crt_frame_label();

        // 将数据上传到 gpu buffer 中
        let cmd = self.frame_ctx.alloc_command_buffer("update-draw-buffer");
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
                time_ms: self.timer.total_time.as_micros() as f32 / 1000.0,
                delta_time_ms: self.timer.elapse_ms(),
                frame_id: self.frame_ctx.crt_frame_id() as u64,
                mouse_pos: shader::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: shader::Float2 {
                    x: frame_extent.width as f32,
                    y: frame_extent.height as f32,
                },
                rt_render_target: self.frame_ctx.crt_frame_bindless_handle(&self.bindless_mgr.borrow()),
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
    // endregion
}
