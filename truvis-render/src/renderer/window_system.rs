use crate::gui::gui::Gui;
use crate::gui::gui_pass::GuiPass;
use crate::pipeline_settings::{DefaultRendererSettings, RendererSettings};
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::PresentData;
use crate::renderer::swapchain::RenderSwapchain;
use ash::vk;
use derive_getters::Getters;
use itertools::Itertools;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::command_pool::RhiCommandPool;
use truvis_rhi::core::command_queue::{RhiQueue, RhiSubmitInfo};
use truvis_rhi::core::image::{RhiImage2D, RhiImage2DView};
use truvis_rhi::core::synchronize::{RhiFence, RhiSemaphore};
use truvis_rhi::rhi::Rhi;
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows, window::Window};

fn load_icon(bytes: &[u8]) -> winit::window::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(bytes).unwrap().into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

pub struct WindowCreateInfo {
    pub width: i32,
    pub height: i32,
    pub title: String,
}

#[derive(Getters)]
pub struct MainWindow {
    window: Window,

    swapchain: RenderSwapchain,
    gui: Gui,

    cmd_buffer: RhiCommandBuffer,
    command_pool: Rc<RhiCommandPool>,
    present_queue: Rc<RhiQueue>,

    width: i32,
    height: i32,

    canvas_depth_image: RhiImage2D,
    canvas_depth_view: RhiImage2DView,

    fence: RhiFence,
    render_complete_semaphores: Vec<RhiSemaphore>,

    gui_pass: GuiPass,
}

impl MainWindow {
    pub fn new(
        event_loop: &ActiveEventLoop,
        rhi: &Rhi,
        create_info: WindowCreateInfo,
        render_settings: &RendererSettings,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
    ) -> Self {
        let icon = load_icon(include_bytes!("../../resources/DruvisIII.png"));
        let window_attr = Window::default_attributes()
            .with_title(create_info.title.clone())
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(f64::from(create_info.width), f64::from(create_info.height)));

        let window = event_loop.create_window(window_attr).unwrap();

        let gui = Gui::new(rhi, &window, render_settings, bindless_mgr.clone());
        let gui_pass = GuiPass::new(rhi, &render_settings.pipeline_settings, bindless_mgr.clone());

        let swapchain = RenderSwapchain::new(
            rhi,
            &window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );
        // TODO 和 swapchain 数量相同
        let render_complete_semaphores =
            (0..3).map(|i| RhiSemaphore::new(rhi, &format!("window-render-complete-{}", i))).collect_vec();

        let present_queue = rhi.present_queue.clone();

        let present_command_pool = Rc::new(RhiCommandPool::new(
            rhi.device.clone(),
            present_queue.queue_family().clone(),
            vk::CommandPoolCreateFlags::empty(),
            "window-present",
        ));

        let cmd_buffer = RhiCommandBuffer::new(rhi.device.clone(), present_command_pool.clone(), "present");

        Self {
            window,
            swapchain,
            render_complete_semaphores,
            gui,
            gui_pass,
            present_queue,
            command_pool: present_command_pool,
            cmd_buffer,
            width: create_info.width,
            height: create_info.height,
            fence: RhiFence::new(rhi, true, "window-acquire-image"),
        }
    }

    pub fn on_window_resize(&mut self, width: u32, height: u32) {
        self.width = width as i32;
        self.height = height as i32;
    }

    pub fn update(&mut self) {
        self.gui.update(&self.window, |ui| todo!("update gui here"));

        todo!()
    }

    fn draw(&mut self, rhi: &Rhi, present_data: Option<&PresentData>) {
        if present_data.is_none() {
            return;
        }
        let present_data = present_data.unwrap();

        self.swapchain.acquire(None, Some(&self.fence), 10 * 1000 * 1000 * 1000);
        self.fence.wait();
        self.fence.reset();

        let canvas_idx = self.swapchain.current_present_image_index();

        self.command_pool.reset_all_buffers();

        let canvas_image = self.swapchain.current_present_image();

        self.cmd_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "window-present");
        // TODO 注意参数来源
        self.gui_pass.draw(
            rhi,
            // TODO 这个不是这个吧
            present_data.image.handle(),
            self.canvas_depth_view.handle(),
            self.swapchain.extent(),
            &self.cmd_buffer,
            &mut self.gui,
        );
        self.cmd_buffer.end();

        let render_complete_semaphore = &self.render_complete_semaphores[canvas_idx];

        // TODO 核实：因为前面 wait fence，因此此处不需要 wait semaphore
        self.present_queue.submit(
            vec![RhiSubmitInfo::new(std::slice::from_ref(&self.cmd_buffer))
                .signal(&present_data.signal_timeline_semaphore, vk::PipelineStageFlags2::FRAGMENT_SHADER, Some(114514))
                .signal(render_complete_semaphore, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, None)],
            None,
        );

        self.swapchain.submit(&self.present_queue, std::slice::from_ref(render_complete_semaphore));
    }
}
