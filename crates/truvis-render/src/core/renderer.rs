use std::ffi::CStr;

use ash::vk;
use truvis_asset::asset_hub::AssetHub;
use truvis_gfx::commands::semaphore::GfxSemaphore;
use truvis_gfx::resources::special_buffers::structured_buffer::GfxStructuredBuffer;
use truvis_gfx::{
    commands::{
        barrier::{GfxBarrierMask, GfxBufferBarrier},
        submit_info::GfxSubmitInfo,
    },
    gfx::Gfx,
};
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_render_base::cmd_allocator::CmdAllocator;
use truvis_render_base::frame_context::FrameContext;
use truvis_render_base::pipeline_settings::AccumData;
use truvis_render_base::stage_buffer_manager::StageBufferManager;
use truvis_render_graph::render_context::{RenderContext, RenderContextMut};
use truvis_render_graph::resources::fif_buffer::FifBuffers;
use truvis_render_scene::gpu_scene::GpuScene;
use truvis_render_scene::scene_manager::SceneManager;
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_shader_binding::truvisl;

use crate::platform::timer::Timer;
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
pub struct Renderer {
    pub render_context: RenderContext,
    pub render_context_mut: RenderContextMut,
    pub asset_hub: AssetHub,
    pub timer: Timer,
}

// new & init
impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        // 初始化 RenderContext 单例
        Gfx::init("Truvis".to_string(), extra_instance_ext);
        FrameContext::init();

        let fif_count = FrameContext::fif_count();
        let timer = Timer::default();
        let accum_data = AccumData::default();
        let fif_timeline_semaphore = GfxSemaphore::new_timeline(0, "render-timeline");

        let mut gfx_resource_manager = GfxResourceManager::new();
        let cmd_allocator = CmdAllocator::new(fif_count);
        let stage_buffer_manager = StageBufferManager::new(fif_count);

        let scene_manager = SceneManager::new();
        let asset_hub = AssetHub::new();
        let mut bindless_manager = BindlessManager::new(fif_count);
        let gpu_scene = GpuScene::new(fif_count, &mut gfx_resource_manager, &mut bindless_manager);
        let fif_buffers = FifBuffers::new(
            &FrameContext::get().frame_settings(),
            &mut bindless_manager,
            &mut gfx_resource_manager,
            fif_count,
        );

        let per_frame_data_buffers = (0..fif_count)
            .map(|idx| GfxStructuredBuffer::<truvisl::PerFrameData>::new_ubo(1, format!("per-frame-data-buffer-{idx}")))
            .collect();

        Self {
            render_context: RenderContext {
                fif_timeline_semaphore,
                scene_manager,
                gpu_scene,
                fif_buffers,
                bindless_manager,
                per_frame_data_buffers,
                gfx_resource_manager,
                delta_time_s: 0.0,
                total_time_s: 0.0,
                accum_data,
            },
            render_context_mut: RenderContextMut {
                cmd_allocator,
                stage_buffer_manager,
            },
            asset_hub,
            timer,
        }
    }
}
// destroy
impl Renderer {
    pub fn destroy(mut self) {
        // 在 Renderer 被销毁时，等待 Gfx 设备空闲
        Gfx::get().wait_idel();

        self.render_context
            .fif_buffers
            .destroy_mut(&mut self.render_context.bindless_manager, &mut self.render_context.gfx_resource_manager);
        FrameContext::destroy();
        self.render_context.bindless_manager.destroy();
        self.render_context.scene_manager.destroy();
        self.asset_hub.destroy();
        self.render_context.gpu_scene.destroy();
        self.render_context_mut.cmd_allocator.destroy();
        self.render_context_mut.stage_buffer_manager.destroy();
        self.render_context.gfx_resource_manager.destroy();
        self.render_context.fif_timeline_semaphore.destroy();
    }
}
// phase call
impl Renderer {
    pub fn begin_frame(&mut self) {
        let _span = tracy_client::span!("Renderer::begin_frame");

        // Update AssetHub
        self.asset_hub.update();

        // 等待 fif 的同一帧渲染完成
        {
            let _span = tracy_client::span!("wait fif timeline");
            let frame_id = FrameContext::frame_id();
            let wait_frame = if frame_id > 3 { frame_id as u64 - 3 } else { 0 };
            let wait_timeline_value = if wait_frame == 0 { 0 } else { wait_frame };
            let timeout_ns = 30 * 1000 * 1000 * 1000;
            self.render_context.fif_timeline_semaphore.wait_timeline(wait_timeline_value, timeout_ns);
        }

        self.render_context_mut.cmd_allocator.free_frame_commands();
        self.render_context_mut.stage_buffer_manager.clear_fif_buffers();
        self.timer.tic();
        self.render_context.delta_time_s = self.timer.delta_time_s();
        self.render_context.total_time_s = self.timer.total_time.as_secs_f32();
    }

    pub fn end_frame(&mut self) {
        let _span = tracy_client::span!("Renderer::end_frame");
        // 设置当前帧结束的 semaphore，用于保护当前帧的资源
        {
            let submit_info = GfxSubmitInfo::new(&[]).signal(
                &self.render_context.fif_timeline_semaphore,
                vk::PipelineStageFlags2::NONE,
                Some(FrameContext::frame_id() as u64),
            );
            Gfx::get().gfx_queue().submit(vec![submit_info], None);
        }

        {
            FrameContext::set_frame_id(FrameContext::frame_id() + 1);
        }
    }

    pub fn time_to_render(&mut self) -> bool {
        let limit_elapsed_us = 1000.0 * 1000.0 / FrameContext::frame_limit();
        limit_elapsed_us < self.timer.toc().as_micros() as f32
    }

    pub fn before_render(&mut self, input_state: &InputState, camera: &Camera) {
        let _span = tracy_client::span!("Renderer::before_render");
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);

        self.render_context.accum_data.update_accum_frames(current_camera_dir, camera.position);
        self.update_gpu_scene(input_state, camera);
    }

    pub fn resize_frame_buffer(&mut self, new_extent: vk::Extent2D) {
        let mut accum_data = self.render_context.accum_data;
        accum_data.reset();

        unsafe {
            Gfx::get().gfx_device().device_wait_idle().unwrap();
        }
        FrameContext::get().set_frame_extent(new_extent);

        self.render_context.fif_buffers.rebuild(
            &mut self.render_context.bindless_manager,
            &mut self.render_context.gfx_resource_manager,
            &FrameContext::get().frame_settings(),
        );
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, camera: &Camera) {
        let _span = tracy_client::span!("update_gpu_scene");
        let frame_extent = FrameContext::get().frame_settings().frame_extent;
        let crt_frame_label = FrameContext::get().frame_label();

        // 将数据上传到 gpu buffer 中
        let cmd = self.render_context_mut.cmd_allocator.alloc_command_buffer("update-draw-buffer");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[update-draw-buffer]stage-to-ubo");

        let transfer_barrier_mask = GfxBarrierMask {
            src_stage: vk::PipelineStageFlags2::TRANSFER,
            src_access: vk::AccessFlags2::TRANSFER_WRITE,
            dst_stage: vk::PipelineStageFlags2::VERTEX_SHADER
                | vk::PipelineStageFlags2::FRAGMENT_SHADER
                | vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
            dst_access: vk::AccessFlags2::SHADER_READ,
        };

        // 因为 Bindless RefCel 的问题，所以这里提前结束作用域
        self.render_context.gpu_scene.prepare_render_data(
            &self.render_context.scene_manager,
            &mut self.render_context.bindless_manager,
            &self.render_context.gfx_resource_manager,
        );
        self.render_context.gpu_scene.upload_to_buffer(
            &cmd,
            transfer_barrier_mask,
            &self.render_context.scene_manager,
            &self.render_context.bindless_manager,
        );

        // 准备好当前帧的数据
        let per_frame_data = {
            let mouse_pos = input_state.crt_mouse_pos;

            let view = camera.get_view_matrix();
            let projection = camera.get_projection_matrix();

            truvisl::PerFrameData {
                projection: projection.into(),
                view: view.into(),
                inv_view: view.inverse().into(),
                inv_projection: projection.inverse().into(),
                camera_pos: camera.position.into(),
                camera_forward: camera.camera_forward().into(),
                time_ms: self.timer.total_time.as_micros() as f32 / 1000.0,
                delta_time_ms: self.timer.delte_time_ms(),
                frame_id: FrameContext::frame_id() as u64,
                mouse_pos: truvisl::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: truvisl::Float2 {
                    x: frame_extent.width as f32,
                    y: frame_extent.height as f32,
                },
                accum_frames: self.render_context.accum_data.accum_frames_num as u32,
                _padding_0: Default::default(),
            }
        };
        let crt_frame_data_buffer = &self.render_context.per_frame_data_buffers[*crt_frame_label];
        cmd.cmd_update_buffer(crt_frame_data_buffer.vk_buffer(), 0, bytemuck::bytes_of(&per_frame_data));
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[GfxBufferBarrier::default()
                .buffer(crt_frame_data_buffer.vk_buffer(), 0, vk::WHOLE_SIZE)
                .mask(transfer_barrier_mask)],
        );
        cmd.end();
        Gfx::get().gfx_queue().submit(vec![GfxSubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }
}
