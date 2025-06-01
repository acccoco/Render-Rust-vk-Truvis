use crate::platform::camera::DrsCamera;
use crate::platform::input_manager::InputState;
use crate::platform::timer::Timer;
use crate::render_context::{FrameSettings, RenderContext};
use crate::render_pass::compute::ComputePass;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::scene_manager::TheWorld;
use crate::renderer::swapchain::RhiSwapchain;
use ash::vk;
use shader_binding::shader;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::synchronize::{RhiBarrierMask, RhiBufferBarrier};
use truvis_rhi::{
    basic::color::LabelColor,
    core::{command_queue::RhiSubmitInfo, synchronize::RhiImageBarrier, window_system::MainWindow},
    rhi::Rhi,
};

const DEPTH_FORMAT_CANDIDATES: &[vk::Format] = &[
    vk::Format::D32_SFLOAT_S8_UINT,
    vk::Format::D32_SFLOAT,
    vk::Format::D24_UNORM_S8_UINT,
    vk::Format::D16_UNORM_S8_UINT,
    vk::Format::D16_UNORM,
];

const FRAMES_IN_FLIGHT: usize = 3;

const DEFAULT_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
    format: vk::Format::B8G8R8A8_UNORM,
    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
};

const DEFAULT_PRESENT_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;

/// 表示整个渲染器进程，需要考虑 platform, render, rhi, log 之类的各种模块
pub struct Renderer {
    /// 需要在 window 存在后创建，且需要手动释放和重新创建，因此使用 Option
    pub render_context: Option<RenderContext>,

    /// 需要在 window 存在后创建，且需要手动释放和重新创建，因此使用 Option
    pub render_swapchain: Option<RhiSwapchain>,

    frame_settings: FrameSettings,

    pub rhi: Rc<Rhi>,

    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub scene_mgr: Rc<RefCell<TheWorld>>,
    pub gpu_scene: GpuScene,
    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,

    blit_pass: ComputePass<shader::blit::PushConstant>,
}
impl Drop for Renderer {
    fn drop(&mut self) {
        log::info!("Dropping Renderer");
        // 在 Renderer 被销毁时，等待 Rhi 设备空闲
        self.wait_idle();

        if let Some(render_context) = self.render_context.take() {
            render_context.destroy(&mut self.bindless_mgr.borrow_mut());
        }
        if let Some(render_swapchain) = self.render_swapchain.take() {
            render_swapchain.destroy(&mut self.bindless_mgr.borrow_mut());
        }
    }
}
// getter
impl Renderer {
    #[inline]
    pub fn swapchain_extent(&self) -> vk::Extent2D {
        self.render_swapchain.as_ref().unwrap().extent()
    }

    #[inline]
    pub fn color_format(&self) -> vk::Format {
        self.render_swapchain.as_ref().unwrap().color_format()
    }

    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
    }

    #[inline]
    pub fn crt_frame_label(&self) -> usize {
        self.render_context.as_ref().unwrap().current_frame_label()
    }

    #[inline]
    pub fn render_context_mut(&mut self) -> &mut RenderContext {
        self.render_context.as_mut().unwrap()
    }

    #[inline]
    pub fn render_context(&self) -> &RenderContext {
        self.render_context.as_ref().unwrap()
    }

    #[inline]
    pub fn swapchain(&self) -> &RhiSwapchain {
        self.render_swapchain.as_ref().unwrap()
    }
}
impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        let rhi = Rc::new(Rhi::new("Truvis".to_string(), extra_instance_ext));

        let bindless_mgr = Rc::new(RefCell::new(BindlessManager::new(&rhi, FRAMES_IN_FLIGHT)));
        let scene_mgr = Rc::new(RefCell::new(TheWorld::new(bindless_mgr.clone())));
        let gpu_scene = GpuScene::new(&rhi, scene_mgr.clone(), bindless_mgr.clone(), FRAMES_IN_FLIGHT);
        let per_frame_data_buffers = (0..FRAMES_IN_FLIGHT)
            .map(|idx| {
                RhiStructuredBuffer::<shader::PerFrameData>::new_ubo(&rhi, 1, format!("per-frame-data-buffer-{idx}"))
            })
            .collect();

        let blit_pass = ComputePass::<shader::blit::PushConstant>::new(
            &rhi,
            &bindless_mgr.borrow(),
            cstr::cstr!("main"),
            "shader/build/imgui/blit.slang.spv",
        );

        Self {
            frame_settings: FrameSettings {
                frames_in_flight: FRAMES_IN_FLIGHT,
                extent: vk::Extent2D::default(),
                rt_rect: vk::Rect2D::default(),
                color_format: DEFAULT_SURFACE_FORMAT.format,
                depth_format: Self::get_depth_format(&rhi),
                accum_frames: None,
                last_camera_dir: glam::Vec3::ZERO,
                last_camera_pos: glam::Vec3::ZERO,
            },
            blit_pass,
            render_context: None,
            render_swapchain: None,
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

    /// 根据 vulkan 实例和显卡，获取合适的深度格式
    fn get_depth_format(rhi: &Rhi) -> vk::Format {
        rhi.find_supported_format(
            DEPTH_FORMAT_CANDIDATES,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
        .first()
        .copied()
        .unwrap_or(vk::Format::UNDEFINED)
    }

    pub fn begin_frame(&mut self) {
        let render_context = self.render_context.as_mut().unwrap();
        let render_swapchain = self.render_swapchain.as_mut().unwrap();

        render_context.begin_frame();
        render_swapchain.acquire(&render_context.current_present_complete_semaphore(), None);
        render_context.before_render(render_swapchain.current_present_image());
    }

    pub fn end_frame(&mut self) {
        // ui pass
        self.rhi.device.debug_utils().begin_queue_label(
            self.rhi.graphics_queue.handle(),
            "[ui-pass]",
            LabelColor::COLOR_PASS,
        );

        let render_context = self.render_context.as_mut().unwrap();
        let render_swapchain = self.render_swapchain.as_mut().unwrap();
        {
            let barrier_cmd = render_context.alloc_command_buffer("ui pipeline barrier");
            barrier_cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[uipass]color-attach-barrier");
            {
                barrier_cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[RhiImageBarrier::new()
                        .image(render_swapchain.current_present_image())
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

        render_context.after_render(render_swapchain.current_present_image());
        render_swapchain.submit(&self.rhi.graphics_queue, &[render_context.current_render_complete_semaphore()]);
        render_context.end_frame();
    }

    pub fn before_render(&mut self, input_state: &InputState, timer: &Timer, camera: &DrsCamera) {
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);
        if camera.position != self.frame_settings.last_camera_pos
            || self.frame_settings.last_camera_dir != current_camera_dir
        {
            self.frame_settings.reset_accum_frames();
        }

        self.frame_settings.last_camera_pos = camera.position;
        self.frame_settings.last_camera_dir = current_camera_dir;

        self.frame_settings.update_accum_frames();
        self.update_gpu_scene(input_state, timer, camera);

        // main pass
        self.rhi.device.debug_utils().begin_queue_label(
            self.rhi.graphics_queue.handle(),
            "[render]",
            LabelColor::COLOR_PASS,
        );
    }

    pub fn after_render(&mut self) {
        self.rhi.device.debug_utils().end_queue_label(self.rhi.graphics_queue.handle());

        // blit
        self.blit();
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }
    }

    /// 在窗口大小改变是，重建 swapchain
    pub fn rebuild_after_resized(&mut self, window: &MainWindow) {
        // 确保 swapchain 已经 drop 掉之后，再创建新的 swapchian，
        // 因为同一时间只能有一个 swapchain 在使用 window
        if let Some(render_context) = self.render_context.take() {
            render_context.destroy(&mut self.bindless_mgr.borrow_mut());
        }
        if let Some(render_swapchain) = self.render_swapchain.take() {
            render_swapchain.destroy(&mut self.bindless_mgr.borrow_mut());
        }

        self.render_swapchain = Some(RhiSwapchain::new(
            &self.rhi,
            window,
            DEFAULT_PRESENT_MODE,
            DEFAULT_SURFACE_FORMAT,
            &mut self.bindless_mgr.borrow_mut(),
        ));

        self.frame_settings.extent = self.swapchain_extent();
        self.frame_settings.rt_rect = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: self.swapchain_extent().width,
                height: self.swapchain_extent().height,
            },
        };
        self.frame_settings.reset_accum_frames();
        self.render_context =
            Some(RenderContext::new(&self.rhi, self.frame_settings, &mut self.bindless_mgr.borrow_mut()));
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, timer: &Timer, camera: &DrsCamera) {
        let render_context = self.render_context.as_mut().unwrap();
        let render_swapchain = self.render_swapchain.as_mut().unwrap();

        let crt_frame_label = render_context.current_frame_label();

        // 将数据上传到 gpu buffer 中
        let cmd = render_context.alloc_command_buffer("update-draw-buffer");
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
            let extent = render_swapchain.extent();

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
                frame_id: render_context.current_frame_num() as u64,
                mouse_pos: shader::Float2 {
                    x: mouse_pos.x as f32,
                    y: mouse_pos.y as f32,
                },
                resolution: shader::Float2 {
                    x: extent.width as f32,
                    y: extent.height as f32,
                },
                rt_render_target: render_context.current_rt_bindless_handle(&self.bindless_mgr.borrow()),
                accum_frames: self.frame_settings.accum_frames.unwrap() as u32,
            }
        };
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
        render_context.graphics_queue().submit(vec![RhiSubmitInfo::new(std::slice::from_ref(&cmd))], None);
    }

    /// 将光追渲染的内容 blit 到 framebuffer 上面
    fn blit(&mut self) {
        let cmd = self.render_context_mut().alloc_command_buffer("blit");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "blit-pass");
        {
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    RhiImageBarrier::new()
                        .image(self.swapchain().current_present_image())
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::GENERAL)
                        .src_mask(
                            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                        )
                        .dst_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_WRITE),
                    RhiImageBarrier::new()
                        .image(self.render_context().current_rt_image().handle())
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .src_mask(
                            vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                            vk::AccessFlags2::SHADER_STORAGE_WRITE,
                        )
                        .dst_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_READ),
                ],
            );

            let rt_image_extent = self.frame_settings.rt_rect.extent;
            let rt_image_offset = self.frame_settings.rt_rect.offset;
            self.blit_pass.exec(
                &cmd,
                &self.bindless_mgr.borrow(),
                &shader::blit::PushConstant {
                    src_image: self.render_context().current_rt_bindless_handle(&self.bindless_mgr.borrow()),
                    dst_image: self.swapchain().current_present_bindless_handle(&self.bindless_mgr.borrow()),
                    src_image_size: glam::uvec2(rt_image_extent.width, rt_image_extent.height).into(),
                    offset: glam::uvec2(rt_image_offset.x as u32, rt_image_offset.y as u32).into(),
                },
                glam::uvec3(
                    rt_image_extent.width.div_ceil(shader::blit::SHADER_X as u32),
                    rt_image_extent.height.div_ceil(shader::blit::SHADER_Y as u32),
                    1,
                ),
            );

            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiImageBarrier::new()
                    .image(self.swapchain().current_present_image())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::GENERAL, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .src_mask(vk::PipelineStageFlags2::COMPUTE_SHADER, vk::AccessFlags2::SHADER_WRITE)
                    .dst_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                    )],
            );
        }
        cmd.end();

        self.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }
}
