use crate::pipeline_settings::{AccumData, DefaultRendererSettings, FrameLabel, FrameSettings};
use crate::platform::camera::DrsCamera;
use crate::platform::input_manager::InputState;
use crate::platform::timer::Timer;
use crate::render_pipeline::pipeline_context::PipelineContext;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_buffers::FrameBuffers;
use crate::renderer::frame_controller::FrameController;
use crate::renderer::gpu_scene::GpuScene;
use crate::renderer::scene_manager::SceneManager;
use ash::vk;
use shader_binding::shader;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;
use truvis_rhi::core::buffer::RhiStructuredBuffer;
use truvis_rhi::core::command_queue::RhiSubmitInfo;
use truvis_rhi::core::descriptor_pool::{RhiDescriptorPool, RhiDescriptorPoolCreateInfo};
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::synchronize::{RhiBarrierMask, RhiBufferBarrier};
use truvis_rhi::core::texture::RhiTexture2D;
use truvis_rhi::rhi::Rhi;

pub struct PresentData<'a> {
    pub render_target: &'a RhiTexture2D,
    pub render_target_bindless_key: String,
    pub render_target_barrier: RhiBarrierMask,
    pub frame_controller: &'a mut FrameController,
}

/// 表示整个渲染器进程，需要考虑 platform, render, rhi, log 之类的各种模块
pub struct Renderer {
    pub rhi: Rc<Rhi>,

    pub frame_ctrl: FrameController,
    framebuffers: FrameBuffers,

    frame_settings: FrameSettings,

    pub bindless_mgr: Rc<RefCell<BindlessManager>>,
    pub scene_mgr: Rc<RefCell<SceneManager>>,
    pub gpu_scene: GpuScene,

    // TODO 优化一下这个 buffer，不该放在这里
    pub per_frame_data_buffers: Vec<RhiStructuredBuffer<shader::PerFrameData>>,
    accum_data: AccumData,

    _descriptor_pool: RhiDescriptorPool,

    timer: Timer,
    fps_limit: f32,
}

// 手动 drop
impl Renderer {
    pub fn destroy(self) {
        // 在 Renderer 被销毁时，等待 Rhi 设备空闲
        self.wait_idle();
        self.frame_ctrl.destroy()
    }
}

// getter
impl Renderer {
    #[inline]
    pub fn frame_settings(&self) -> FrameSettings {
        self.frame_settings
    }

    pub fn deltatime(&self) -> std::time::Duration {
        self.timer.delta_time
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

    pub fn get_renderer_data(&mut self) -> PresentData {
        let crt_frame_label = self.frame_ctrl.frame_label();

        let (render_target, render_target_bindless_key) = self.framebuffers.render_target_texture(crt_frame_label);
        PresentData {
            render_target,
            render_target_bindless_key,
            render_target_barrier: RhiBarrierMask {
                src_stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                src_access: vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
                dst_stage: vk::PipelineStageFlags2::NONE,
                dst_access: vk::AccessFlags2::NONE,
            },
            frame_controller: &mut self.frame_ctrl,
        }
    }

    #[inline]
    pub fn frame_controller(&self) -> &FrameController {
        &self.frame_ctrl
    }
}

// init
impl Renderer {
    pub fn new(extra_instance_ext: Vec<&'static CStr>) -> Self {
        let rhi = Rc::new(Rhi::new("Truvis".to_string(), extra_instance_ext));

        let descriptor_pool = Self::init_descriptor_pool(rhi.device.clone());

        let bindless_mgr =
            Rc::new(RefCell::new(BindlessManager::new(&rhi, &descriptor_pool, FrameLabel::FRAMES_IN_FLIGHT)));
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
        let frame_ctrl = FrameController::new(&rhi, &frame_settings);
        let framebuffers = FrameBuffers::new(&rhi, &frame_settings, &mut bindless_mgr.borrow_mut());

        Self {
            frame_settings,
            framebuffers,
            accum_data: Default::default(),
            frame_ctrl,
            rhi,
            bindless_mgr,
            scene_mgr,
            gpu_scene,
            per_frame_data_buffers,
            timer: Timer::default(),
            _descriptor_pool: descriptor_pool,
            fps_limit: 59.9,
        }
    }

    const DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_MATERIAL_CNT: u32 = 256;
    const DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT: u32 = 128;

    fn init_descriptor_pool(device: Rc<RhiDevice>) -> RhiDescriptorPool {
        let pool_size = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
                descriptor_count: 128,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::INPUT_ATTACHMENT,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
                descriptor_count: 32,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: Self::DESCRIPTOR_POOL_MAX_BINDLESS_TEXTURE_CNT + 32,
            },
        ];

        let pool_ci = Rc::new(RhiDescriptorPoolCreateInfo::new(
            vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            Self::DESCRIPTOR_POOL_MAX_MATERIAL_CNT + Self::DESCRIPTOR_POOL_MAX_VERTEX_BLENDING_MESH_CNT + 32,
            pool_size,
        ));

        RhiDescriptorPool::new(device, pool_ci, "renderer")
    }
}

// phase call
impl Renderer {
    pub fn begin_frame(&mut self) {
        self.frame_ctrl.begin_frame();
        self.timer.tic();
    }

    pub fn end_frame(&mut self) {
        self.frame_ctrl.end_frame(&self.rhi);
    }

    pub fn time_to_render(&self) -> bool {
        let limit_elapsed_us = 1000.0 * 1000.0 / self.fps_limit;
        limit_elapsed_us < self.timer.toc().as_micros() as f32
    }

    pub fn before_render(&mut self, input_state: &InputState, camera: &DrsCamera) {
        let current_camera_dir = glam::vec3(camera.euler_yaw_deg, camera.euler_pitch_deg, camera.euler_roll_deg);
        self.accum_data.update_accum_frames(current_camera_dir, camera.position);
        self.update_gpu_scene(input_state, camera);
    }

    pub fn after_render(&mut self) {
        self.frame_ctrl.end_render(&self.rhi);
    }

    pub fn wait_idle(&self) {
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }
    }

    pub fn collect_render_ctx(&mut self) -> PipelineContext {
        let crt_frame_label = self.frame_ctrl.frame_label();

        PipelineContext {
            rhi: &self.rhi,
            gpu_scene: &self.gpu_scene,
            bindless_mgr: self.bindless_mgr.clone(),
            per_frame_data: &self.per_frame_data_buffers[*crt_frame_label],
            frame_ctrl: &mut self.frame_ctrl,
            timer: &self.timer,
            frame_settings: &self.frame_settings,
            frame_buffers: &self.framebuffers,
        }
    }

    pub fn resize_frame_buffer(&mut self, new_extent: vk::Extent2D) {
        self.accum_data.reset();
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }
        self.frame_settings.frame_extent = new_extent;
        self.framebuffers.rebuild(&self.rhi, &self.frame_settings, &mut self.bindless_mgr.borrow_mut());
    }

    fn update_gpu_scene(&mut self, input_state: &InputState, camera: &DrsCamera) {
        let frame_extent = self.frame_settings.frame_extent;
        let crt_frame_label = self.frame_ctrl.frame_label();

        // 将数据上传到 gpu buffer 中
        let cmd = self.frame_ctrl.alloc_command_buffer("update-draw-buffer");
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
