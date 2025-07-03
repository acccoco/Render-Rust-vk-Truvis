use winit::event_loop::ActiveEventLoop;
use ash::vk;
use std::rc::Rc;
use std::cell::RefCell;
use truvis_rhi::rhi::Rhi;
use winit::window::Window;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::command_pool::RhiCommandPool;
use truvis_rhi::core::command_queue::{RhiQueue, RhiSubmitInfo};
use truvis_rhi::core::synchronize::{RhiFence, RhiImageBarrier, RhiSemaphore};
use winit::platform::windows::WindowAttributesExtWindows;
use itertools::Itertools;
use crate::gui::gui::Gui;
use crate::gui::gui_pass::GuiPass;
use crate::pipeline_settings::DefaultRendererSettings;
use crate::platform::timer::Timer;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_controller::RendererData;
use crate::renderer::swapchain::RenderSwapchain;

mod helper {
    pub fn load_icon(bytes: &[u8]) -> winit::window::Icon {
        let (icon_rgba, icon_width, icon_height) = {
            let image = image::load_from_memory(bytes).unwrap().into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };
        winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
    }
}

pub struct MainWindow {
    winit_window: Window,

    swapchain: Option<RenderSwapchain>,
    gui: Gui,
    gui_pass: GuiPass,

    cmd_buffer: RhiCommandBuffer,
    command_pool: Rc<RhiCommandPool>,
    present_queue: Rc<RhiQueue>,

    render_complete_fence: RhiFence,
    /// 只需要一个，由 timeline 来确保其 signal 和 wait 都已经完成
    present_complete_semaphore: RhiSemaphore,
    /// 表示 gui 的绘制已经完成；
    ///
    /// 数量和 swapchain 的 image 数量相同，
    /// 因为每个 image 都需要一个对应的 semaphore 来等待 gui 绘制完成后再进行呈现
    ///
    /// timeline 可以确保 signal 已经完成，但是无法确保 present 操作对其的 wait 已经完成
    render_complete_semaphores: Vec<RhiSemaphore>,

    timer: Timer,
    fps_limit: f32,
    frame_id: u64,
}

impl MainWindow {
    pub fn new(
        event_loop: &ActiveEventLoop,
        rhi: &Rhi,
        window_title: String,
        window_extent: vk::Extent2D,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        let icon = helper::load_icon(include_bytes!("../../resources/DruvisIII.png"));
        let window_attr = Window::default_attributes()
            .with_title(window_title)
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(window_extent.width as f64, window_extent.height as f64));

        let window = event_loop.create_window(window_attr).unwrap();
        let swapchain = RenderSwapchain::new(
            rhi,
            &window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );

        let present_settings = swapchain.present_settings();

        let gui = Gui::new(rhi, &window, &present_settings, bindless_mgr.clone());
        let gui_pass = GuiPass::new(rhi, bindless_mgr.clone(), present_settings.color_format);

        let present_complete_semaphore = RhiSemaphore::new(rhi, "window-present-complete");
        let render_complete_semaphores = (0..present_settings.swapchain_image_cnt)
            .map(|i| RhiSemaphore::new(rhi, &format!("window-render-complete-{}", i)))
            .collect_vec();

        let present_queue = rhi.present_queue.clone();

        let present_command_pool = Rc::new(RhiCommandPool::new(
            rhi.device.clone(),
            present_queue.queue_family().clone(),
            vk::CommandPoolCreateFlags::empty(),
            "window-present",
        ));

        let cmd_buffer = RhiCommandBuffer::new(rhi.device.clone(), present_command_pool.clone(), "present");

        Self {
            winit_window: window,
            swapchain: Some(swapchain),
            present_complete_semaphore,
            render_complete_semaphores,
            gui,
            gui_pass,
            present_queue,
            command_pool: present_command_pool,
            cmd_buffer,
            render_complete_fence: RhiFence::new(rhi, true, "window-acquire-image"),
            timer: Timer::default(),
            fps_limit: 59.9,
            frame_id: 1,
        }
    }

    #[inline]
    pub fn window(&self) -> &Window {
        &self.winit_window
    }

    pub fn time_to_update(&self) -> bool {
        let limit_elapsed_us = 1000.0 * 1000.0 / self.fps_limit;
        self.timer.toc().as_micros() as f32 > limit_elapsed_us
    }

    pub fn update(
        &mut self,
        rhi: &Rhi,
        ui_func: impl FnOnce(&mut imgui::Ui),
        renderer_data: Option<RendererData>,
    ) -> Option<u64> {
        let elapsed = self.timer.toc();
        self.timer.tic();
        let rtn = renderer_data.as_ref().map(|_| self.frame_id);

        self.gui.prepare_frame(&self.winit_window, elapsed);
        // TODO 这里将 render-image 的 bindless-key 送入 gui 中，让 gui 记住，然后在 gui-draw 时可以找到这个 image，再去 bindless-mgr 中获取 handle
        self.gui.update(&self.winit_window, renderer_data.as_ref().map(|d| d.image_bindless_key.clone()), ui_func);
        self.draw(rhi, renderer_data);

        self.frame_id += 1;
        rtn
    }

    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.gui.handle_event(&self.winit_window, event);
    }

    /// imgui 中用于绘制图形的区域大小
    pub fn get_render_extent(&self) -> vk::Extent2D {
        self.gui.get_render_region().extent
    }

    pub fn rebuild_after_resized(&mut self, rhi: &Rhi) {
        self.swapchain = None;

        self.swapchain = Some(RenderSwapchain::new(
            rhi,
            &self.winit_window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        ));
    }

    fn draw(&mut self, rhi: &Rhi, renderer_data: Option<RendererData>) {
        // 直接阻塞等待，确保上一帧的 command buffer，present complete semaphore 都是空闲的
        {
            self.render_complete_fence.wait();
            self.render_complete_fence.reset();
            self.command_pool.reset_all_buffers();
        }

        // 从 swapchain 获取图像
        let swapchain = self.swapchain.as_mut().unwrap();
        let timeout_ns = 10 * 1000 * 1000 * 1000;
        swapchain.acquire_next_image(Some(&self.present_complete_semaphore), None, timeout_ns);
        let canvas_idx = swapchain.current_image_index();

        self.cmd_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "window-present");

        // 将 image layout 转换为 COLOR_ATTACHMENT_OPTIMAL
        // 注：这里 bottom 是必须的
        // 可能有 blend 操作，因此需要 COLOR_ATTACHMENT_READ
        self.cmd_buffer.image_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiImageBarrier::new()
                .image(swapchain.current_image())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                )],
        );

        // TODO 注意参数来源
        self.gui_pass.draw(
            rhi,
            swapchain.current_image_view().handle(),
            swapchain.extent(),
            &self.cmd_buffer,
            &mut self.gui,
        );
        self.cmd_buffer.end();

        self.cmd_buffer.image_memory_barrier(
            vk::DependencyFlags::empty(),
            &[RhiImageBarrier::new()
                .image(swapchain.current_image())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR)
                .src_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                )
                .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())],
        );

        let render_complete_semaphore = &self.render_complete_semaphores[canvas_idx];

        // 等待 swapchain 的 image 准备好；通知 swapchain 的 image 已经绘制完成
        let mut submit_info = RhiSubmitInfo::new(std::slice::from_ref(&self.cmd_buffer))
            .wait(&self.present_complete_semaphore, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, None)
            .signal(render_complete_semaphore, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, None);

        // 等待 renderer 数据的绘制完成；通知 renderer 数据的绘制已经使用完毕
        if let Some(renderer_data) = renderer_data {
            submit_info = submit_info
                .wait(
                    &renderer_data.wait_timeline_semaphore,
                    vk::PipelineStageFlags2::FRAGMENT_SHADER,
                    Some(renderer_data.wait_timeline_value),
                )
                .signal(
                    &renderer_data.signal_timeline_semaphore,
                    vk::PipelineStageFlags2::FRAGMENT_SHADER,
                    Some(self.frame_id),
                );
        }
        self.present_queue.submit(vec![submit_info], Some(self.render_complete_fence.clone()));

        swapchain.present_image(&self.present_queue, std::slice::from_ref(render_complete_semaphore));
    }
}