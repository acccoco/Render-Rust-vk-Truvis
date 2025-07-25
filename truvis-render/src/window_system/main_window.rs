use crate::gui::gui::Gui;
use crate::gui::gui_pass::GuiPass;
use crate::pipeline_settings::{DefaultRendererSettings, FrameLabel};
use crate::renderer::bindless::BindlessManager;
use crate::renderer::renderer::PresentData;
use crate::renderer::swapchain::RenderSwapchain;
use ash::vk;
use itertools::Itertools;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_rhi::core::command_queue::RhiSubmitInfo;
use truvis_rhi::core::synchronize::{RhiImageBarrier, RhiSemaphore};
use truvis_rhi::rhi::Rhi;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::Window;

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
    rhi: Rc<Rhi>,
    winit_window: Window,

    swapchain: Option<RenderSwapchain>,
    gui: Gui,
    gui_pass: GuiPass,

    /// 数量和 fif num 相同
    present_complete_semaphores: Vec<RhiSemaphore>,

    /// 表示 gui 的绘制已经完成；
    ///
    /// 数量和 swapchain 的 image 数量相同，
    /// 因为每个 image 都需要一个对应的 semaphore 来等待 gui 绘制完成后再进行呈现
    ///
    /// renderer 的 wait timeline 可以确保 signal 操作已经完成，但是无法 wait 操作已经完成
    render_complete_semaphores: Vec<RhiSemaphore>,
}

// ctor
impl MainWindow {
    pub fn new(
        event_loop: &ActiveEventLoop,
        rhi: Rc<Rhi>,
        fif_num: usize,
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
            &rhi,
            &window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );

        let present_settings = swapchain.present_settings();

        let gui = Gui::new(&rhi, &window, fif_num, &present_settings, bindless_mgr.clone());
        let gui_pass = GuiPass::new(&rhi, bindless_mgr.clone(), present_settings.color_format);

        let present_complete_semaphores =
            (0..fif_num).map(|i| RhiSemaphore::new(&rhi, &format!("window-present-complete-{}", i))).collect_vec();
        let render_complete_semaphores = (0..present_settings.swapchain_image_cnt)
            .map(|i| RhiSemaphore::new(&rhi, &format!("window-render-complete-{}", i)))
            .collect_vec();

        Self {
            rhi,
            winit_window: window,
            swapchain: Some(swapchain),
            present_complete_semaphores,
            render_complete_semaphores,
            gui,
            gui_pass,
        }
    }

    #[inline]
    pub fn window(&self) -> &Window {
        &self.winit_window
    }

    fn draw(&mut self, renderer_data: PresentData) {
        let swapchain = self.swapchain.as_ref().unwrap();
        let canvas_idx = swapchain.current_image_index();
        let frame_label = renderer_data.frame_controller.frame_label();

        let cmd = renderer_data.frame_controller.alloc_command_buffer("window-present");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "window-present");
        {
            // 将 swapchian image layout 转换为 COLOR_ATTACHMENT_OPTIMAL
            // 注1: 可能有 blend 操作，因此需要 COLOR_ATTACHMENT_READ
            // 注2: 这里的 bottom 表示 layout transfer 等待 present 完成
            let swapchain_image_layout_transfer_barrier = RhiImageBarrier::new()
                .image(swapchain.current_image())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                );

            let render_target_barrier = RhiImageBarrier::new()
                .image(renderer_data.render_target.image())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(renderer_data.render_target_barrier.src_stage, renderer_data.render_target_barrier.src_access)
                .dst_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ);

            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[swapchain_image_layout_transfer_barrier, render_target_barrier],
            );

            self.gui_pass.draw(
                &self.rhi,
                swapchain.current_image_view().handle(),
                swapchain.extent(),
                &cmd,
                &mut self.gui,
                frame_label,
            );

            // 将 swapchain image layout 转换为 PRESENT_SRC_KHR
            // 注1: 这里的 top 表示 present 需要等待 layout transfer 完成
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[RhiImageBarrier::new()
                    .image(swapchain.current_image())
                    .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR)
                    .src_mask(
                        vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                        vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                    )
                    .dst_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())],
            );
        }
        cmd.end();

        // 等待 swapchain 的 image 准备好；通知 swapchain 的 image 已经绘制完成
        let submit_info = RhiSubmitInfo::new(std::slice::from_ref(&cmd))
            .wait(
                &self.present_complete_semaphores[*frame_label],
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                None,
            )
            .signal(
                &self.render_complete_semaphores[canvas_idx],
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                None,
            );

        self.rhi.graphics_queue.submit(vec![submit_info], None);
    }
}

// 手动 drop
impl MainWindow {
    pub fn destroy(self) {
        for semaphore in self.present_complete_semaphores {
            semaphore.destroy();
        }
        for semaphore in self.render_complete_semaphores {
            semaphore.destroy();
        }
    }
}

// phase
impl MainWindow {
    pub fn acquire_image(&mut self, frame_label: FrameLabel) {
        // 从 swapchain 获取图像
        let swapchain = self.swapchain.as_mut().unwrap();
        // let timeout_ns = 10 * 1000 * 1000 * 1000;
        swapchain.acquire_next_image(Some(&self.present_complete_semaphores[*frame_label]), None, 0);
    }

    pub fn present_image(&self) {
        let swapchain = self.swapchain.as_ref().unwrap();
        swapchain.present_image(
            &self.rhi.graphics_queue,
            std::slice::from_ref(&self.render_complete_semaphores[swapchain.current_image_index()]),
        );
    }

    pub fn update_gui(&mut self, elapsed: std::time::Duration, ui_func_right: impl FnOnce(&imgui::Ui)) {
        self.gui.prepare_frame(&self.winit_window, elapsed);
        self.gui.update(
            &self.winit_window,
            |ui, content_size| {
                let min_pos = ui.window_content_region_min();
                ui.set_cursor_pos([min_pos[0] + 5.0, min_pos[1] + 5.0]);
                ui.text(format!("FPS: {:.2}", 1.0 / elapsed.as_secs_f32()));
                ui.text(format!("size: {:.0}x{:.0}", content_size[0], content_size[1]));
            },
            ui_func_right,
        );
    }

    pub fn draw_gui(&mut self, renderer_data: PresentData) {
        self.gui.register_render_image_key(renderer_data.render_target_bindless_key.clone());
        self.draw(renderer_data);
    }

    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.gui.handle_event(&self.winit_window, event);
    }

    /// imgui 中用于绘制图形的区域大小
    pub fn get_render_extent(&self) -> vk::Extent2D {
        self.gui.get_render_region().extent
    }

    pub fn rebuild_after_resized(&mut self) {
        unsafe {
            self.rhi.device.device_wait_idle().unwrap();
        }

        self.swapchain = None;
        self.swapchain = Some(RenderSwapchain::new(
            &self.rhi,
            &self.winit_window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        ));
    }
}
