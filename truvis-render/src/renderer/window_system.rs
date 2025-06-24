use crate::gui::gui::Gui;
use crate::gui::gui_pass::GuiPass;
use crate::pipeline_settings::{DefaultRendererSettings, FrameSettings, PresentSettings, RendererSettings};
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::RendererData;
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

#[derive(Getters)]
pub struct MainWindow {
    window: Window,

    swapchain: Option<RenderSwapchain>,
    gui: Gui,

    cmd_buffer: RhiCommandBuffer,
    command_pool: Rc<RhiCommandPool>,
    present_queue: Rc<RhiQueue>,

    fence: RhiFence,
    render_complete_semaphores: Vec<RhiSemaphore>,

    gui_pass: GuiPass,
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

        // TODO 和 swapchain 数量相同
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
            window,
            swapchain: Some(swapchain),
            render_complete_semaphores,
            gui,
            gui_pass,
            present_queue,
            command_pool: present_command_pool,
            cmd_buffer,
            fence: RhiFence::new(rhi, true, "window-acquire-image"),
        }
    }

    pub fn update(&mut self) {
        self.gui.update(&self.window, |ui| todo!("update gui here"));

        todo!()
    }

    pub fn rebuild_after_resized(&mut self, rhi: &Rhi) {
        self.swapchain = None;

        self.swapchain = Some(RenderSwapchain::new(
            rhi,
            &self.window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        ));
    }

    fn draw(&mut self, rhi: &Rhi, renderer_data: Option<&RendererData>) {
        if renderer_data.is_none() {
            return;
        }
        let renderer_data = renderer_data.unwrap();

        let swapchain = self.swapchain.as_mut().unwrap();

        swapchain.acquire(None, Some(&self.fence), 10 * 1000 * 1000 * 1000);
        self.fence.wait();
        self.fence.reset();

        let canvas_idx = swapchain.current_present_image_index();

        self.command_pool.reset_all_buffers();

        let canvas_image = swapchain.current_present_image();

        self.cmd_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "window-present");
        // TODO 注意参数来源
        self.gui_pass.draw(
            rhi,
            // TODO 这个不是这个吧
            renderer_data.image.handle(),
            swapchain.extent(),
            &self.cmd_buffer,
            &mut self.gui,
        );
        self.cmd_buffer.end();

        let render_complete_semaphore = &self.render_complete_semaphores[canvas_idx];

        // TODO 核实：因为前面 wait fence，因此此处不需要 wait semaphore
        self.present_queue.submit(
            vec![RhiSubmitInfo::new(std::slice::from_ref(&self.cmd_buffer))
                .signal(
                    &renderer_data.signal_timeline_semaphore,
                    vk::PipelineStageFlags2::FRAGMENT_SHADER,
                    Some(114514),
                )
                .signal(render_complete_semaphore, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, None)],
            None,
        );

        swapchain.submit(&self.present_queue, std::slice::from_ref(render_complete_semaphore));
    }
}
