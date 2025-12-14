use ash::vk;
use itertools::Itertools;
use winit::{event_loop::ActiveEventLoop, platform::windows::WindowAttributesExtWindows, window::Window};

use crate::gui::core::Gui;
use crate::gui::gui_pass::GuiPass;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::commands::barrier::GfxBarrierMask;
use truvis_gfx::{
    commands::{barrier::GfxImageBarrier, semaphore::GfxSemaphore, submit_info::GfxSubmitInfo},
    gfx::Gfx,
    swapchain::render_swapchain::GfxRenderSwapchain,
};
use truvis_render::core::renderer::Renderer;
use truvis_render_base::frame_counter::FrameCounter;
use truvis_render_base::pipeline_settings::{DefaultRendererSettings, FrameLabel};
use truvis_render_graph::render_context::{RenderContext, RenderContextMut};
use truvis_resource::handles::GfxTextureHandle;

/// 渲染演示数据结构
///
/// 包含了向演示窗口提交渲染结果所需的所有数据和资源。
/// 这个结构体作为渲染器内部状态与外部演示系统之间的桥梁。
pub struct PresentData {
    /// 当前帧的渲染目标纹理
    ///
    /// 包含了最终的渲染结果，将被复制或演示到屏幕上
    pub render_target: GfxTextureHandle,

    /// 渲染目标的内存屏障配置
    ///
    /// 定义了渲染目标纹理的同步需求，确保在读取前所有写入操作已完成
    pub render_target_barrier: GfxBarrierMask,
}

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

    swapchain: Option<GfxRenderSwapchain>,
    gui: Gui,
    gui_pass: GuiPass,

    /// 数量和 fif num 相同
    present_complete_semaphores: Vec<GfxSemaphore>,

    /// 表示 gui 的绘制已经完成；
    ///
    /// 数量和 swapchain 的 image 数量相同，
    /// 因为每个 image 都需要一个对应的 semaphore 来等待 gui
    /// 绘制完成后再进行呈现
    ///
    /// renderer 的 wait timeline 可以确保 signal 操作已经完成，但是无法 wait
    /// 操作已经完成
    render_complete_semaphores: Vec<GfxSemaphore>,
}

// ctor
impl MainWindow {
    pub fn new(
        renderer: &mut Renderer,
        event_loop: &ActiveEventLoop,
        window_title: String,
        window_extent: vk::Extent2D,
    ) -> Self {
        let icon_data = std::fs::read(TruvisPath::resources_path("DruvisIII.png")).expect("Failed to read icon file");
        let icon = helper::load_icon(icon_data.as_ref());
        let window_attr = Window::default_attributes()
            .with_title(window_title)
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon.clone()))
            .with_transparent(true)
            .with_inner_size(winit::dpi::LogicalSize::new(window_extent.width as f64, window_extent.height as f64));

        let window = event_loop.create_window(window_attr).unwrap();
        let swapchain = GfxRenderSwapchain::new(
            Gfx::get().vk_core(),
            &window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );

        let swapchain_image_infos = swapchain.image_infos();

        let gui = Gui::new(renderer, &window, FrameCounter::fif_count(), &swapchain_image_infos);
        let gui_pass = GuiPass::new(&renderer.render_context.bindless_manager, swapchain_image_infos.image_format);

        let present_complete_semaphores = (0..FrameCounter::fif_count())
            .map(|i| GfxSemaphore::new(&format!("window-present-complete-{}", i)))
            .collect_vec();
        let render_complete_semaphores = (0..swapchain_image_infos.image_cnt)
            .map(|i| GfxSemaphore::new(&format!("window-render-complete-{}", i)))
            .collect_vec();

        Self {
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

    fn draw(
        &mut self,
        render_context: &RenderContext,
        render_context_mut: &mut RenderContextMut,
        renderer_data: PresentData,
    ) {
        let swapchain = self.swapchain.as_ref().unwrap();
        let swapchain_image_idx = swapchain.current_image_index();
        let frame_label = render_context.frame_counter.frame_label();

        let render_target_texture =
            render_context.gfx_resource_manager.get_texture(renderer_data.render_target).unwrap();

        let cmd =
            render_context_mut.cmd_allocator.alloc_command_buffer(&render_context.frame_counter, "window-present");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "window-present");
        {
            // 将 swapchian image layout 转换为 COLOR_ATTACHMENT_OPTIMAL
            // 注1: 可能有 blend 操作，因此需要 COLOR_ATTACHMENT_READ
            // 注2: 这里的 bottom 表示 layout transfer 等待 present 完成
            let swapchain_image_layout_transfer_barrier = GfxImageBarrier::new()
                .image(swapchain.current_image())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                .dst_mask(
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                );

            let render_target_barrier = GfxImageBarrier::new()
                .image(render_target_texture.image().handle())
                .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                .src_mask(renderer_data.render_target_barrier.src_stage, renderer_data.render_target_barrier.src_access)
                .dst_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ);

            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[swapchain_image_layout_transfer_barrier, render_target_barrier],
            );

            self.gui_pass.draw(
                render_context,
                render_context_mut,
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
                &[GfxImageBarrier::new()
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
        let submit_info = GfxSubmitInfo::new(std::slice::from_ref(&cmd))
            .wait(
                &self.present_complete_semaphores[*frame_label],
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                None,
            )
            .signal(
                &self.render_complete_semaphores[swapchain_image_idx],
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                None,
            );

        Gfx::get().gfx_queue().submit(vec![submit_info], None);
    }
}

// destroy
impl MainWindow {
    pub fn destroy(self) {
        for semaphore in self.present_complete_semaphores {
            semaphore.destroy();
        }
        for semaphore in self.render_complete_semaphores {
            semaphore.destroy();
        }
        if let Some(swapchain) = self.swapchain {
            swapchain.destroy();
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
            Gfx::get().gfx_queue(),
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

    pub fn draw_gui(
        &mut self,
        render_context: &RenderContext,
        render_context_mut: &mut RenderContextMut,
        renderer_data: PresentData,
    ) {
        self.gui.register_render_texture(renderer_data.render_target);
        self.draw(render_context, render_context_mut, renderer_data);
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
            Gfx::get().gfx_device().device_wait_idle().unwrap();
        }

        if let Some(swapchain) = self.swapchain.take() {
            swapchain.destroy();
        }
        self.swapchain = Some(GfxRenderSwapchain::new(
            Gfx::get().vk_core(),
            &self.winit_window,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        ));
    }
}
