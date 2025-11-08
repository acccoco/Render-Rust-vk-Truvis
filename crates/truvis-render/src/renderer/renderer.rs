use crate::renderer::frame_context::FrameContext;
use crate::{
    pipeline_settings::{AccumData, DefaultRendererSettings, FrameSettings, PipelineSettings},
    platform::{camera::Camera, input_manager::InputState, timer::Timer},
    render_pipeline::pipeline_context::PipelineContext,
    renderer::{
        frame_buffers::FrameBuffers, frame_controller::FrameController, gpu_scene::GpuScene,
        scene_manager::SceneManager,
    },
};
use ash::vk;
use std::{cell::RefCell, ffi::CStr, rc::Rc};
use truvis_gfx::{
    commands::{
        barrier::{BarrierMask, BufferBarrier},
        semaphore::Semaphore,
        submit_info::SubmitInfo,
    },
    render_context::RenderContext,
    resources::{special_buffers::structured_buffer::StructuredBuffer, texture::Texture2D},
};
use truvis_shader_binding::shader;

/// 渲染演示数据结构
///
/// 包含了向演示窗口提交渲染结果所需的所有数据和资源。
/// 这个结构体作为渲染器内部状态与外部演示系统之间的桥梁。
pub struct PresentData<'a> {
    /// 当前帧的渲染目标纹理
    ///
    /// 包含了最终的渲染结果，将被复制或演示到屏幕上
    pub render_target: &'a Texture2D,

    /// 渲染目标在 Bindless 系统中的唯一标识符
    ///
    /// 用于在着色器中通过 Bindless 方式访问渲染目标纹理
    pub render_target_bindless_key: String,

    /// 渲染目标的内存屏障配置
    ///
    /// 定义了渲染目标纹理的同步需求，确保在读取前所有写入操作已完成
    pub render_target_barrier: BarrierMask,
}

/// 表示整个渲染器进程，需要考虑 platform, render, render_context, log 之类的各种模块
pub struct Renderer {
    // TODO 移除 Renderer::frame_ctrl，直接通过 FrameContext 获取
    pub frame_ctrl: Rc<FrameController>,
    framebuffers: FrameBuffers,

    frame_settings: FrameSettings,
    pipeline_settings: PipelineSettings,

    pub scene_mgr: RefCell<SceneManager>,
    pub gpu_scene: GpuScene,

    // TODO 优化一下这个 buffer，不该放在这里
    pub per_frame_data_buffers: Vec<StructuredBuffer<shader::PerFrameData>>,
    accum_data: AccumData,

    /// 帧渲染完成的 timeline，value 就等于 frame_id
    render_timeline_semaphore: Semaphore,

    timer: Timer,
    fps_limit: f32,
}

// 手动 drop
impl Renderer {
    pub fn destroy(self) {
        // 在 Renderer 被销毁时，等待 Gfx 设备空闲
        self.wait_idle();
        self.render_timeline_semaphore.destroy();
    }
}

// getter
impl Renderer {
    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
    }

    #[inline]
    pub fn pipeline_settings(&mut self) -> &mut PipelineSettings {
        &mut self.pipeline_settings
    }

    #[inline]
    pub fn accum_frames(&self) -> usize {
        self.accum_data.accum_frames_num
    }

    pub fn deltatime(&self) -> std::time::Duration {
        self.timer.delta_time
    }

    /// 根据 vulkan 实例和显卡，获取合适的深度格式
    fn get_depth_format() -> vk::Format {
        RenderContext::get()
            .find_supported_format(
                DefaultRendererSettings::DEPTH_FORMAT_CANDIDATES,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )
            .first()
            .copied()
            .unwrap_or(vk::Format::UNDEFINED)
    }

    pub fn get_renderer_data(&mut self) -> PresentData<'_> {
        let crt_frame_label = self.frame_ctrl.frame_label();

        let (render_target, render_target_bindless_key) = self.framebuffers.render_target_texture(crt_frame_label);
        PresentData {
            render_target,
            render_target_bindless_key,
            render_target_barrier: BarrierMask {
                src_stage: vk::PipelineStageFlags2::COMPUTE_SHADER,
                src_access: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                dst_stage: vk::PipelineStageFlags2::NONE,
                dst_access: vk::AccessFlags2::NONE,
            },
        }
    }

    #[inline]
    pub fn frame_controller(&self) -> &FrameController {
        &self.frame_ctrl
    }
}

// tools
impl Renderer {}

// init
impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        // 初始化 RenderContext 单例
        RenderContext::init("Truvis".to_string(), extra_instance_ext);
        FrameContext::init();

        let frame_settings = FrameSettings {
            color_format: vk::Format::R32G32B32A32_SFLOAT,
            depth_format: Self::get_depth_format(),
            frame_extent: vk::Extent2D {
                width: 400,
                height: 400,
            },
        };
        let frame_ctrl = FrameContext::get().frame_ctrl.clone();

        let scene_mgr = RefCell::new(SceneManager::new());
        let gpu_scene = GpuScene::new(frame_ctrl.clone());

        // 注册 GPU 场景使用的默认纹理
        gpu_scene.register_default_textures();

        let per_frame_data_buffers = (0..frame_ctrl.fif_count())
            .map(|idx| StructuredBuffer::<shader::PerFrameData>::new_ubo(1, format!("per-frame-data-buffer-{idx}")))
            .collect();

        let framebuffers = FrameBuffers::new(&frame_settings, frame_ctrl.clone());

        let render_timeline_semaphore = Semaphore::new_timeline(0, "render-timeline");

        Self {
            frame_settings,
            pipeline_settings: PipelineSettings::default(),
            framebuffers,
            accum_data: Default::default(),
            frame_ctrl,
            scene_mgr,
            gpu_scene,
            per_frame_data_buffers,
            timer: Timer::default(),
            fps_limit: 59.9,
            render_timeline_semaphore,
        }
    }
}

// phase call
impl Renderer {
    pub fn begin_frame(&mut self) {
        // 等待 fif 的同一帧渲染完成
        {
            let frame_id = self.frame_ctrl.frame_id();
            let wait_frame = if frame_id > 3 { frame_id as u64 - 3 } else { 0 };
            let wait_timeline_value = if wait_frame == 0 { 0 } else { wait_frame };
            let timeout_ns = 30 * 1000 * 1000 * 1000;
            self.render_timeline_semaphore.wait_timeline(wait_timeline_value, timeout_ns);
        }

        FrameContext::cmd_allocator_mut().free_frame_commands();
        FrameContext::stage_buffer_manager().clear_frame_buffers();
        self.timer.tic();
    }

    pub fn end_frame(&mut self) {
        // 设置当前帧结束的 semaphore，用于保护当前帧的资源
        {
            let submit_info = SubmitInfo::new(&[]).signal(
                &self.render_timeline_semaphore,
                vk::PipelineStageFlags2::NONE,
                Some(self.frame_ctrl.frame_id() as u64),
            );
            RenderContext::get().gfx_queue().submit(vec![submit_info], None);
        }

        self.frame_ctrl.end_frame();
    }

    pub fn time_to_render(&self) -> bool {
        let limit_elapsed_us = 1000.0 * 1000.0 / self.fps_limit;
        limit_elapsed_us < self.timer.toc().as_micros() as f32
    }

    pub fn before_render(&mut self, input_state: &InputState, camera: &Camera) {
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);
        self.accum_data.update_accum_frames(current_camera_dir, camera.position);
        self.update_gpu_scene(input_state, camera);
    }

    pub fn after_render(&mut self) {}

    pub fn wait_idle(&self) {
        unsafe {
            RenderContext::get().device_functions().device_wait_idle().unwrap();
        }
    }

    pub fn collect_render_ctx(&mut self) -> PipelineContext<'_> {
        let crt_frame_label = self.frame_ctrl.frame_label();

        PipelineContext {
            gpu_scene: &self.gpu_scene,
            per_frame_data: &self.per_frame_data_buffers[*crt_frame_label],
            timer: &self.timer,
            frame_settings: &self.frame_settings,
            pipeline_settings: &self.pipeline_settings,
            frame_buffers: &self.framebuffers,
        }
    }

    pub fn resize_frame_buffer(&mut self, new_extent: vk::Extent2D) {
        self.accum_data.reset();
        unsafe {
            RenderContext::get().device_functions().device_wait_idle().unwrap();
        }
        self.frame_settings.frame_extent = new_extent;
        self.framebuffers.rebuild(&self.frame_settings);
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, camera: &Camera) {
        let frame_extent = self.frame_settings.frame_extent;
        let crt_frame_label = self.frame_ctrl.frame_label();

        // 将数据上传到 gpu buffer 中
        let cmd = FrameContext::cmd_allocator_mut().alloc_command_buffer("update-draw-buffer");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[update-draw-buffer]stage-to-ubo");

        let transfer_barrier_mask = BarrierMask {
            src_stage: vk::PipelineStageFlags2::TRANSFER,
            src_access: vk::AccessFlags2::TRANSFER_WRITE,
            dst_stage: vk::PipelineStageFlags2::VERTEX_SHADER
                | vk::PipelineStageFlags2::FRAGMENT_SHADER
                | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            dst_access: vk::AccessFlags2::SHADER_READ,
        };

        self.gpu_scene.prepare_render_data(&self.scene_mgr.borrow());
        self.gpu_scene.upload_to_buffer(&cmd, transfer_barrier_mask, &self.scene_mgr.borrow());

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
                delta_time_ms: self.timer.delte_time_ms(),
                frame_id: self.frame_ctrl.frame_id() as u64,
                mouse_pos: shader::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: shader::Float2 {
                    x: frame_extent.width as f32,
                    y: frame_extent.height as f32,
                },
                accum_frames: self.accum_data.accum_frames_num as u32,
                _padding_0: Default::default(),
            }
        };
        cmd.cmd_update_buffer(
            self.per_frame_data_buffers[*crt_frame_label].vk_buffer(),
            0,
            bytemuck::bytes_of(&per_frame_data),
        );
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[BufferBarrier::default()
                .buffer(self.per_frame_data_buffers[*crt_frame_label].vk_buffer(), 0, vk::WHOLE_SIZE)
                .mask(transfer_barrier_mask)],
        );
        cmd.end();
        RenderContext::get().gfx_queue().submit(vec![SubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }
}
