use crate::frame_context::{FrameContext, RenderContextInitInfo};
use crate::platform::camera::TruCamera;
use crate::platform::input_manager::InputState;
use crate::platform::timer::Timer;
use crate::renderer::acc_manager::AccManager;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::scene_manager::TheWorld;
use ash::vk;
use raw_window_handle::HasDisplayHandle;
use shader_binding::shader;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::synchronize::{RhiBarrierMask, RhiBufferBarrier};
use truvis_rhi::{
    basic::color::LabelColor,
    core::{
        command_queue::RhiSubmitInfo, swapchain::RhiSwapchainInitInfo, synchronize::RhiImageBarrier,
        window_system::MainWindow,
    },
    rhi::Rhi,
};

/// 表示整个渲染器进程，需要考虑 platform, render, rhi, log 之类的各种模块
pub struct Renderer {
    /// window 需要在 event loop 中创建，因此使用 option 包装
    pub window: Rc<MainWindow>,

    /// render context 需要在 event loop 中创建，因此使用 option 包装
    ///
    /// 依赖于 window
    pub render_context: FrameContext,

    /// Rhi 需要在 window 之后创建，因为需要获取 window 相关的 extension
    pub rhi: Rc<Rhi>,

    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub scene_mgr: Rc<RefCell<TheWorld>>,
    pub acc_mgr: Rc<RefCell<AccManager>>,
    pub gpu_scene: GpuScene,
    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,
}
impl Drop for Renderer {
    fn drop(&mut self) {
        log::info!("Dropping Renderer");
        // 在 Renderer 被销毁时，等待 Rhi 设备空闲
        self.wait_idle();
    }
}
impl Renderer {
    pub fn new(window_system: Rc<MainWindow>) -> Self {
        // rhi
        let rhi = {
            // 追加 window system 需要的 extension，在 windows 下也就是 khr::Surface
            let extra_instance_ext =
                ash_window::enumerate_required_extensions(window_system.window().display_handle().unwrap().as_raw())
                    .unwrap()
                    .iter()
                    .map(|ext| unsafe { CStr::from_ptr(*ext) })
                    .collect();
            Rc::new(Rhi::new("Truvis".to_string(), extra_instance_ext))
        };

        // render context
        let render_context = {
            let render_swapchain_init_info = RhiSwapchainInitInfo::new(window_system.clone());

            let render_context_init_info = RenderContextInitInfo::default();
            FrameContext::new(&rhi, &render_context_init_info, render_swapchain_init_info)
        };

        let frames_in_flight = render_context.frame_cnt_in_flight;

        let bindless_mgr = Rc::new(RefCell::new(BindlessManager::new(&rhi, render_context.frame_cnt_in_flight)));
        let scene_mgr = Rc::new(RefCell::new(TheWorld::new(bindless_mgr.clone())));
        let acc_mgr = Rc::new(RefCell::new(AccManager::new(&rhi, frames_in_flight)));
        let gpu_scene = GpuScene::new(&rhi, scene_mgr.clone(), bindless_mgr.clone(), acc_mgr.clone(), frames_in_flight);
        let per_frame_data_buffers = (0..frames_in_flight)
            .map(|idx| {
                RhiStructuredBuffer::<shader::PerFrameData>::new_ubo(&rhi, 1, format!("per-frame-data-buffer-{idx}"))
            })
            .collect();

        Self {
            window: window_system,
            render_context,
            rhi,
            bindless_mgr,
            scene_mgr,
            gpu_scene,
            acc_mgr,
            per_frame_data_buffers,
        }
    }

    pub fn before_frame(&mut self) {
        self.render_context.acquire_frame();
    }

    pub fn after_frame(&mut self) {
        // ui pass
        self.rhi.device.debug_utils().begin_queue_label(
            self.rhi.graphics_queue.handle(),
            "[ui-pass]",
            LabelColor::COLOR_PASS,
        );
        {
            let barrier_cmd = self.render_context.alloc_command_buffer("ui pipeline barrier");
            barrier_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[uipass]color-attach-barrier");
            {
                barrier_cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[RhiImageBarrier::new()
                        .image(self.render_context.current_present_image())
                        .layout_transfer(
                            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        )
                        .src_mask(
                            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        )
                        .dst_mask(
                            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        )
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)],
                );
            }
            barrier_cmd.end();

            self.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[barrier_cmd])], None);
        }
        self.rhi.device.debug_utils().end_queue_label(self.rhi.graphics_queue.handle());

        self.render_context.submit_frame();
    }

    pub fn before_render(&mut self) {
        // main pass
        self.rhi.device.debug_utils().begin_queue_label(
            self.rhi.graphics_queue.handle(),
            "[render]",
            LabelColor::COLOR_PASS,
        );
    }

    pub fn after_render(&mut self) {
        self.rhi.device.debug_utils().end_queue_label(self.rhi.graphics_queue.handle());
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }
    }

    /// 在窗口大小改变是，重建 swapchain
    pub fn rebuild_render_context(&mut self) {
        // 需要先销毁旧的 RenderContext，然后再创建新的 RenderContext。
        // 如果直接使用 self.render_context = FrameContext::new(...)
        // 会导致新的 RenderContext 先被创建，老的 RenderContext 才会被 drop
        // 然而仅允许有一个 Swapchain 存在，因此需要先销毁旧的 RenderContext，再创建新的 RenderContext。

        // 首先获取旧的 render_context，将其从 self 中取出
        let old_render_context = unsafe {
            // 使用 std::ptr::read 从 self.render_context 的位置读取值
            // 这样不会调用任何 drop 函数，只是简单地移走值
            std::ptr::read(&self.render_context)
        };

        // 显式调用 drop 以确保资源被正确释放
        drop(old_render_context);

        // 创建新的 render_context
        let render_swapchain_init_info = RhiSwapchainInitInfo::new(self.window.clone());
        let render_context_init_info = RenderContextInitInfo::default();
        let new_render_context = FrameContext::new(&self.rhi, &render_context_init_info, render_swapchain_init_info);

        // 安全地放入新的 render_context，不会在旧位置调用 drop
        unsafe {
            // 使用 std::ptr::write 直接写入新值，不会调用任何 drop
            std::ptr::write(&mut self.render_context, new_render_context);
        }
    }

    pub fn update_gpu_scene(&mut self, input_state: &InputState, timer: &Timer, camera: &TruCamera) {
        let crt_frame_label = self.render_context.current_frame_label();

        // 准备好当前帧的数据
        let per_frame_data = {
            let mouse_pos = input_state.crt_mouse_pos;
            let extent = self.render_context.swapchain_extent();

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
                frame_id: self.render_context.current_frame_num() as u64,
                mouse_pos: shader::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: shader::Float2 {
                    x: extent.width as f32,
                    y: extent.height as f32,
                },
                ..Default::default()
            }
        };

        // 将数据上传到 gpu buffer 中
        let cmd = self.render_context.alloc_command_buffer("update-draw-buffer");
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
        self.gpu_scene.upload_to_buffer(
            &self.rhi,
            crt_frame_label,
            &cmd,
            transfer_barrier_mask,
            self.render_context.current_rt_image_view(),
        );

        cmd.cmd_update_buffer(
            self.per_frame_data_buffers[crt_frame_label].handle(),
            0,
            bytemuck::bytes_of(&per_frame_data),
        );
        cmd.buffer_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiBufferBarrier::default()
                .buffer(self.per_frame_data_buffers[crt_frame_label].handle(), 0, vk::WHOLE_SIZE)
                .mask(transfer_barrier_mask)],
        );
        cmd.end();
        self.render_context.graphics_queue().submit(vec![RhiSubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }
}
