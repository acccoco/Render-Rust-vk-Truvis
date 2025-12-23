use crate::present::gui_backend::GuiBackend;
use crate::present::gui_pass::GuiPass;
use ash::vk;
use imgui::DrawData;
use itertools::Itertools;
use truvis_gfx::commands::barrier::{GfxBarrierMask, GfxImageBarrier};
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::commands::semaphore::GfxSemaphore;
use truvis_gfx::commands::submit_info::GfxSubmitInfo;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::swapchain::render_swapchain::GfxRenderSwapchain;
use truvis_render_base::cmd_allocator::CmdAllocator;
use truvis_render_base::frame_counter::FrameCounter;
use truvis_render_base::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_base::pipeline_settings::{DefaultRendererSettings, FrameLabel};
use truvis_render_graph::render_context::RenderContext;
use truvis_resource::handles::GfxTextureHandle;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

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

pub struct RenderPresent {
    pub swapchain: Option<GfxRenderSwapchain>,
    pub gui_backend: GuiBackend,
    pub gui_pass: GuiPass,

    /// 数量和 fif num 相同
    pub present_complete_semaphores: [GfxSemaphore; FrameCounter::fif_count()],

    /// 数量和 swapchain image num 相同
    pub render_complete_semaphores: Vec<GfxSemaphore>,
}
// new & init
impl RenderPresent {
    pub fn new(
        global_descriptor_sets: &GlobalDescriptorSets,
        cmd_allocator: &mut CmdAllocator,
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
    ) -> Self {
        let swapchain = GfxRenderSwapchain::new(
            Gfx::get().vk_core(),
            raw_display_handle,
            raw_window_handle,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        );

        let swapchain_image_infos = swapchain.image_infos();

        let gui_backend = GuiBackend::new(cmd_allocator);
        let gui_pass = GuiPass::new(&global_descriptor_sets, swapchain_image_infos.image_format);

        let present_complete_semaphores = FrameCounter::frame_labes()
            .map(|frame_label| GfxSemaphore::new(&format!("window-present-complete-{}", frame_label)));
        let render_complete_semaphores = (0..swapchain_image_infos.image_cnt)
            .map(|i| GfxSemaphore::new(&format!("window-render-complete-{}", i)))
            .collect_vec();

        Self {
            swapchain: Some(swapchain),
            gui_backend,
            gui_pass,
            present_complete_semaphores,
            render_complete_semaphores,
        }
    }
}
// update
impl RenderPresent {
    pub fn rebuild_after_resized(&mut self, raw_display_handle: RawDisplayHandle, raw_window_handle: RawWindowHandle) {
        unsafe {
            Gfx::get().gfx_device().device_wait_idle().unwrap();
        }

        if let Some(swapchain) = self.swapchain.take() {
            swapchain.destroy();
        }
        self.swapchain = Some(GfxRenderSwapchain::new(
            Gfx::get().vk_core(),
            raw_display_handle,
            raw_window_handle,
            DefaultRendererSettings::DEFAULT_PRESENT_MODE,
            DefaultRendererSettings::DEFAULT_SURFACE_FORMAT,
        ));
    }

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

    pub fn draw_gui(
        &mut self,
        render_context: &RenderContext,
        draw_data: Option<&DrawData>,
        present_data: PresentData,
    ) {
        self.gui_backend.register_render_texture(present_data.render_target);

        self.draw(render_context, draw_data, present_data);
    }

    fn draw(&mut self, render_context: &RenderContext, draw_data: Option<&DrawData>, present_data: PresentData) {
        let swapchain = self.swapchain.as_ref().unwrap();
        let swapchain_image_idx = swapchain.current_image_index();
        let frame_label = render_context.frame_counter.frame_label();

        let render_target_texture =
            render_context.gfx_resource_manager.get_texture(present_data.render_target).unwrap();

        let cmd = self.gui_backend.cmds[*frame_label].clone();
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "window-present");
        {
            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    // 将 swapchian image layout 转换为 COLOR_ATTACHMENT_OPTIMAL
                    // 注1: 可能有 blend 操作，因此需要 COLOR_ATTACHMENT_READ
                    // 注2: 这里的 bottom 表示 layout transfer 等待 present 完成
                    GfxImageBarrier::new()
                        .image(swapchain.current_image())
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .src_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                        .dst_mask(
                            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                        ),
                    GfxImageBarrier::new()
                        .image(render_target_texture.image().handle())
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .layout_transfer(vk::ImageLayout::GENERAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .src_mask(
                            present_data.render_target_barrier.src_stage,
                            present_data.render_target_barrier.src_access,
                        )
                        .dst_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ),
                ],
            );

            if let Some(draw_data) = draw_data {
                self.gui_backend.prepare_render_data(draw_data, render_context.frame_counter.frame_label());

                self.gui_pass.draw(
                    render_context,
                    swapchain.current_image_view().handle(),
                    swapchain.extent(),
                    &cmd,
                    frame_label,
                    &self.gui_backend.gui_meshes[*frame_label],
                    draw_data,
                    &self.gui_backend.tex_map,
                );
            }

            cmd.image_memory_barrier(
                vk::DependencyFlags::empty(),
                &[
                    // 将 swapchain image layout 转换为 PRESENT_SRC_KHR
                    // 注1: 这里的 top 表示 present 需要等待 layout transfer 完成
                    GfxImageBarrier::new()
                        .image(swapchain.current_image())
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .layout_transfer(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR)
                        .src_mask(
                            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE | vk::AccessFlags2::COLOR_ATTACHMENT_READ,
                        )
                        .dst_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty()),
                    GfxImageBarrier::new()
                        .image(render_target_texture.image().handle())
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .layout_transfer(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::ImageLayout::GENERAL)
                        .src_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER, vk::AccessFlags2::SHADER_READ)
                        .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty()),
                ],
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
impl RenderPresent {
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
