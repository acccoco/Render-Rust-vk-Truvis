use std::rc::{Rc, Weak};

use ash::vk;
use itertools::Itertools;

use truvis_gfx::{
    commands::barrier::ImageBarrier,
    gfx::Gfx,
    resources::{
        image::{Image2D, ImageCreateInfo},
        image_view::{Image2DView, ImageViewCreateInfo},
        texture::Texture2D,
    },
};
use truvis_shader_binding::shader;

use crate::core::frame_context::FrameContext;
use crate::{
    pipeline_settings::{FrameLabel, FrameSettings},
    subsystems::bindless_manager::BindlessManager,
};

/// 所有帧会用到的 buffers
pub struct FifBuffers {
    /// RT 计算的累积结果
    accum_image: Rc<Image2D>,
    accum_image_view: Rc<Image2DView>,

    _depth_image: Rc<Image2D>,
    depth_image_view: Rc<Image2DView>,

    /// 离屏渲染的结果，数量和 fif 相同
    off_screen_targets: Vec<Rc<Texture2D>>,
    off_screen_target_bindless_keys: Vec<String>,
}
impl FifBuffers {
    pub fn new(frame_settigns: &FrameSettings, bindless_manager: &mut BindlessManager, fif_count: usize) -> Self {
        let (color_image, color_image_view) = Self::create_color_image(frame_settigns);
        let (depth_image, depth_image_view) = Self::create_depth_image(frame_settigns);
        let render_targets = Self::create_render_targets(frame_settigns, fif_count);
        let render_target_bindless_keys = render_targets
            .iter()
            .enumerate()
            .map(|(i, _)| format!("render-target-{}", FrameLabel::from_usize(i)))
            .collect_vec();

        let fif_buffers = Self {
            accum_image: color_image,
            accum_image_view: color_image_view,
            _depth_image: depth_image,
            depth_image_view,
            off_screen_targets: render_targets,
            off_screen_target_bindless_keys: render_target_bindless_keys,
        };
        fif_buffers.register_bindless(bindless_manager);
        fif_buffers
    }

    /// 尺寸发生变化时，需要重新创建相关的资源
    pub fn rebuild(&mut self, frame_settings: &FrameSettings) {
        self.unregister_bindless();
        *self = Self::new(frame_settings, &mut FrameContext::bindless_manager_mut(), FrameContext::get().fif_count());
    }

    fn register_bindless(&self, bindless_manager: &mut BindlessManager) {
        bindless_manager.register_image(&self.accum_image_view);
        for (render_target, key) in self.off_screen_targets.iter().zip(self.off_screen_target_bindless_keys.iter()) {
            bindless_manager.register_texture_shared(key.clone(), render_target.clone());
            bindless_manager.register_image(render_target.image_view());
        }
    }

    fn unregister_bindless(&self) {
        let mut bindless_manager = FrameContext::bindless_manager_mut();

        bindless_manager.unregister_image2(&self.accum_image_view);
        for (render_target, key) in self.off_screen_targets.iter().zip(self.off_screen_target_bindless_keys.iter()) {
            bindless_manager.unregister_texture(key);
            bindless_manager.unregister_image2(render_target.image_view());
        }
    }

    /// 创建 RayTracing 需要的 image
    fn create_color_image(frame_settings: &FrameSettings) -> (Rc<Image2D>, Rc<Image2DView>) {
        let color_image = Rc::new(Image2D::new(
            Rc::new(ImageCreateInfo::new_image_2d_info(
                frame_settings.frame_extent,
                frame_settings.color_format,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "fif-buffer-color",
        ));

        let color_image_view = Rc::new(Image2DView::new(
            color_image.handle(),
            ImageViewCreateInfo::new_image_view_2d_info(frame_settings.color_format, vk::ImageAspectFlags::COLOR),
            "fif-buffer-color",
        ));

        // layout transfer
        Gfx::get().one_time_exec(
            |cmd| {
                cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[ImageBarrier::new()
                        .image(color_image.handle())
                        .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                        .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                        .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)],
                );
            },
            "transfer-fif-buffer-color-image-layout",
        );

        (color_image, color_image_view)
    }

    fn create_depth_image(frame_settings: &FrameSettings) -> (Rc<Image2D>, Rc<Image2DView>) {
        let depth_image = Rc::new(Image2D::new(
            Rc::new(ImageCreateInfo::new_image_2d_info(
                frame_settings.frame_extent,
                frame_settings.depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "fif-buffer-depth",
        ));

        let depth_image_view = Image2DView::new(
            depth_image.handle(),
            ImageViewCreateInfo::new_image_view_2d_info(frame_settings.depth_format, vk::ImageAspectFlags::DEPTH),
            "fif-buffer-depth",
        );

        (depth_image, Rc::new(depth_image_view))
    }

    fn create_render_targets(frame_settings: &FrameSettings, fif_count: usize) -> Vec<Rc<Texture2D>> {
        let create_texture = |fif_labe: FrameLabel| {
            let name = format!("render-target-{}", fif_labe);
            let color_image = Rc::new(Image2D::new(
                Rc::new(ImageCreateInfo::new_image_2d_info(
                    frame_settings.frame_extent,
                    frame_settings.color_format,
                    vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::SAMPLED
                        | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                )),
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    ..Default::default()
                },
                &name,
            ));
            Texture2D::new(color_image, &name)
        };

        (0..fif_count)
            .map(|fif_label| {
                let texture = create_texture(FrameLabel::from_usize(fif_label));
                Rc::new(texture)
            })
            .collect_vec()
    }
}
// getter
impl FifBuffers {
    #[inline]
    pub fn depth_image_view(&self) -> &Image2DView {
        &self.depth_image_view
    }

    #[inline]
    pub fn render_target_texture(&self, frame_label: FrameLabel) -> (Weak<Texture2D>, String) {
        (
            Rc::downgrade(&self.off_screen_targets[frame_label as usize]),
            self.off_screen_target_bindless_keys[frame_label as usize].clone(),
        )
    }

    #[inline]
    pub fn render_target_image(&self, frame_label: FrameLabel) -> vk::Image {
        self.off_screen_targets[frame_label as usize].image()
    }

    #[inline]
    pub fn render_target_image_view(&self, frame_label: FrameLabel) -> &Image2DView {
        self.off_screen_targets[frame_label as usize].image_view()
    }

    pub fn color_image_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        bindless_manager.get_image_handle(&self.accum_image_view).unwrap()
    }

    #[inline]
    pub fn color_image(&self) -> &Image2D {
        &self.accum_image
    }

    #[inline]
    pub fn color_image_view(&self) -> &Image2DView {
        &self.accum_image_view
    }

    pub fn render_target_image_bindless_handle(
        &self,
        bindless_manager: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::ImageHandle {
        bindless_manager.get_image_handle(self.off_screen_targets[frame_label as usize].image_view()).unwrap()
    }

    pub fn render_target_texture_bindless_handle(
        &self,
        bindless_manager: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::TextureHandle {
        bindless_manager.get_texture_handle(&self.off_screen_target_bindless_keys[frame_label as usize]).unwrap()
    }
}
impl Drop for FifBuffers {
    fn drop(&mut self) {
        // RAII
    }
}
