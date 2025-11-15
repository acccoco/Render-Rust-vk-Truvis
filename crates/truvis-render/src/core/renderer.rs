use std::ffi::CStr;

use ash::vk;

use truvis_gfx::{
    commands::{
        barrier::{BarrierMask, BufferBarrier},
        submit_info::SubmitInfo,
    },
    gfx::Gfx,
};
use truvis_shader_binding::shader;

use crate::core::frame_context::FrameContext;
use crate::platform::{camera::Camera, input_manager::InputState};

/// 渲染器核心
///
/// 管理整个渲染流程，包括帧同步、资源更新、GPU 场景同步等。
/// 与 [`FrameContext`] 配合工作，提供帧级生命周期管理。
///
/// # 渲染流程
/// ```ignore
/// renderer.begin_frame();        // 等待 GPU、清理资源
/// // OuterApp::update() / OuterApp::draw()
/// renderer.before_render();      // 更新相机、输入状态
/// // 录制命令...
/// renderer.end_frame();          // 提交命令、推进帧计数
/// ```
pub struct Renderer {}

// 手动 drop
impl Renderer {
    pub fn destroy(self) {
        // 在 Renderer 被销毁时，等待 Gfx 设备空闲
        Gfx::get().wait_idel();
    }
}

// tools
impl Renderer {}

// init
impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        // 初始化 RenderContext 单例
        Gfx::init("Truvis".to_string(), extra_instance_ext);
        FrameContext::init();

        // 注册 GPU 场景使用的默认纹理
        FrameContext::gpu_scene_mut().register_default_textures();

        Self {}
    }
}

// phase call
impl Renderer {
    pub fn begin_frame(&mut self) {
        // 等待 fif 的同一帧渲染完成
        {
            let frame_id = FrameContext::get().frame_id();
            let wait_frame = if frame_id > 3 { frame_id as u64 - 3 } else { 0 };
            let wait_timeline_value = if wait_frame == 0 { 0 } else { wait_frame };
            let timeout_ns = 30 * 1000 * 1000 * 1000;
            FrameContext::get().fif_timeline_semaphore.wait_timeline(wait_timeline_value, timeout_ns);
        }

        FrameContext::cmd_allocator_mut().free_frame_commands();
        FrameContext::stage_buffer_manager().clear_fif_buffers();
        FrameContext::get().timer.borrow_mut().tic();
    }

    pub fn end_frame(&mut self) {
        // 设置当前帧结束的 semaphore，用于保护当前帧的资源
        {
            let submit_info = SubmitInfo::new(&[]).signal(
                &FrameContext::get().fif_timeline_semaphore,
                vk::PipelineStageFlags2::NONE,
                Some(FrameContext::get().frame_id() as u64),
            );
            Gfx::get().gfx_queue().submit(vec![submit_info], None);
        }

        FrameContext::get().end_frame();
    }

    pub fn time_to_render(&self) -> bool {
        let limit_elapsed_us = 1000.0 * 1000.0 / FrameContext::get().frame_limit;
        limit_elapsed_us < FrameContext::get().timer.borrow_mut().toc().as_micros() as f32
    }

    pub fn before_render(&mut self, input_state: &InputState, camera: &Camera) {
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);

        let mut accum_data = FrameContext::get().accum_data.get();
        accum_data.update_accum_frames(current_camera_dir, camera.position);
        FrameContext::get().accum_data.set(accum_data);

        self.update_gpu_scene(input_state, camera);
    }

    pub fn resize_frame_buffer(&mut self, new_extent: vk::Extent2D) {
        let mut accum_data = FrameContext::get().accum_data.get();
        accum_data.reset();
        FrameContext::get().accum_data.set(accum_data);

        unsafe {
            Gfx::get().gfx_device().device_wait_idle().unwrap();
        }
        FrameContext::get().set_frame_extent(new_extent);
        FrameContext::get().fif_buffers.borrow_mut().rebuild(&FrameContext::get().frame_settings());
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, camera: &Camera) {
        let frame_extent = FrameContext::get().frame_settings().frame_extent;
        let crt_frame_label = FrameContext::get().frame_label();

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

        let mut gpu_scene = FrameContext::gpu_scene_mut();
        gpu_scene.prepare_render_data(&FrameContext::scene_manager());
        gpu_scene.upload_to_buffer(&cmd, transfer_barrier_mask, &FrameContext::scene_manager());

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
                time_ms: FrameContext::get().timer.borrow().total_time.as_micros() as f32 / 1000.0,
                delta_time_ms: FrameContext::get().timer.borrow().delte_time_ms(),
                frame_id: FrameContext::get().frame_id() as u64,
                mouse_pos: shader::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: shader::Float2 {
                    x: frame_extent.width as f32,
                    y: frame_extent.height as f32,
                },
                accum_frames: FrameContext::get().accum_data.get().accum_frames_num as u32,
                _padding_0: Default::default(),
            }
        };
        let crt_frame_data_buffer = &FrameContext::get().per_frame_data_buffers[*crt_frame_label];
        cmd.cmd_update_buffer(crt_frame_data_buffer.vk_buffer(), 0, bytemuck::bytes_of(&per_frame_data));
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[BufferBarrier::default()
                .buffer(crt_frame_data_buffer.vk_buffer(), 0, vk::WHOLE_SIZE)
                .mask(transfer_barrier_mask)],
        );
        cmd.end();
        Gfx::get().gfx_queue().submit(vec![SubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }
}
