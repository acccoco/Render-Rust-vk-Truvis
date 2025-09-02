use std::rc::Rc;

use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use truvis_rhi::{
    commands::{barrier::ImageBarrier, command_buffer::CommandBuffer},
    resources::{
        image::{Image2D, ImageCreateInfo},
        image_view::{Image2DView, ImageViewCreateInfo},
        texture::Texture2D,
    },
    render_context::RenderContext,
};

use crate::{
    pipeline_settings::{FrameLabel, FrameSettings},
    renderer::{bindless::BindlessManager, frame_controller::FrameController},
};

/// 所有帧会用到的 buffers
pub struct FrameBuffers
{
    color_image: Rc<Image2D>,
    color_image_view: Rc<Image2DView>,

    _depth_image: Rc<Image2D>,
    depth_image_view: Rc<Image2DView>,

    /// fif 每一帧的渲染结果
    render_targets: Vec<Rc<Texture2D>>,
    render_target_bindless_keys: Vec<String>,

    frame_ctrl: Rc<FrameController>,
}

impl FrameBuffers
{
    pub fn new(
        rhi: &RenderContext,
        frame_settigns: &FrameSettings,
        frame_ctrl: Rc<FrameController>,
        bindless_mgr: &mut BindlessManager,
    ) -> Self
    {
        let (color_image, color_image_view) = Self::create_color_image(rhi, frame_settigns);
        let (depth_image, depth_image_view) = Self::create_depth_image(rhi, frame_settigns);
        let render_targets = Self::create_render_targets(rhi, frame_settigns, &frame_ctrl);
        let render_target_bindless_keys = render_targets
            .iter()
            .enumerate()
            .map(|(i, _)| format!("render-target-{}", FrameLabel::from_usize(i)))
            .collect_vec();

        let framebuffers = Self {
            color_image,
            color_image_view,
            _depth_image: depth_image,
            depth_image_view,
            render_targets,
            render_target_bindless_keys,
            frame_ctrl,
        };
        framebuffers.register_bindless(bindless_mgr);
        framebuffers
    }

    pub fn rebuild(&mut self, rhi: &RenderContext, frame_settings: &FrameSettings, bindless_mgr: &mut BindlessManager)
    {
        self.unregister_bindless(bindless_mgr);
        *self = Self::new(rhi, frame_settings, self.frame_ctrl.clone(), bindless_mgr);
    }

    fn register_bindless(&self, bindless_mgr: &mut BindlessManager)
    {
        bindless_mgr.register_image_shared(self.color_image_view.clone());

        for (render_target, key) in self.render_targets.iter().zip(self.render_target_bindless_keys.iter()) {
            bindless_mgr.register_texture_shared(key.clone(), render_target.clone());
            bindless_mgr.register_image_raw(render_target.image_view());
        }
    }

    fn unregister_bindless(&self, bindless_mgr: &mut BindlessManager)
    {
        bindless_mgr.unregister_image(&self.color_image_view.uuid());

        for (render_target, key) in self.render_targets.iter().zip(self.render_target_bindless_keys.iter()) {
            bindless_mgr.unregister_texture(key);
            bindless_mgr.unregister_image(&render_target.image_view().uuid())
        }
    }

    /// 创建 RayTracing 需要的 image
    fn create_color_image(rhi: &RenderContext, frame_settings: &FrameSettings) -> (Rc<Image2D>, Rc<Image2DView>)
    {
        let color_image = Rc::new(Image2D::new(
            rhi,
            Rc::new(ImageCreateInfo::new_image_2d_info(
                frame_settings.frame_extent,
                frame_settings.color_format,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "framebuffer-color",
        ));

        let color_image_view = Rc::new(Image2DView::new(
            rhi,
            color_image.handle(),
            ImageViewCreateInfo::new_image_view_2d_info(frame_settings.color_format, vk::ImageAspectFlags::COLOR),
            "framebuffer-color",
        ));

        // layout transfer
        rhi.one_time_exec(
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
            "transfer-framebuffer-color-image-layout",
        );

        (color_image, color_image_view)
    }

    fn create_depth_image(rhi: &RenderContext, frame_settings: &FrameSettings) -> (Rc<Image2D>, Rc<Image2DView>)
    {
        let depth_image = Rc::new(Image2D::new(
            rhi,
            Rc::new(ImageCreateInfo::new_image_2d_info(
                frame_settings.frame_extent,
                frame_settings.depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "framebuffer-depth",
        ));

        let depth_image_view = Image2DView::new(
            rhi,
            depth_image.handle(),
            ImageViewCreateInfo::new_image_view_2d_info(frame_settings.depth_format, vk::ImageAspectFlags::DEPTH),
            "framebuffer-depth",
        );

        (depth_image, Rc::new(depth_image_view))
    }

    fn create_render_targets(
        rhi: &RenderContext,
        frame_settings: &FrameSettings,
        frame_ctrl: &FrameController,
    ) -> Vec<Rc<Texture2D>>
    {
        let create_texture = |fif_labe: FrameLabel| {
            let name = format!("render-target-{}", fif_labe);
            let color_image = Rc::new(Image2D::new(
                rhi,
                Rc::new(ImageCreateInfo::new_image_2d_info(
                    frame_settings.frame_extent,
                    frame_settings.color_format,
                    vk::ImageUsageFlags::STORAGE |
                        vk::ImageUsageFlags::TRANSFER_SRC |
                        vk::ImageUsageFlags::SAMPLED |
                        vk::ImageUsageFlags::COLOR_ATTACHMENT,
                )),
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    ..Default::default()
                },
                &name,
            ));
            Texture2D::new(rhi, color_image, &name)
        };

        (0..frame_ctrl.fif_count())
            .map(|fif_label| {
                let texture = create_texture(FrameLabel::from_usize(fif_label));
                Rc::new(texture)
            })
            .collect_vec()
    }
}

// getter
impl FrameBuffers
{
    #[inline]
    pub fn depth_image_view(&self) -> &Image2DView
    {
        &self.depth_image_view
    }

    #[inline]
    pub fn render_target_texture(&self, frame_label: FrameLabel) -> (&Texture2D, String)
    {
        (
            &self.render_targets[frame_label as usize], //
            self.render_target_bindless_keys[frame_label as usize].clone(),
        )
    }

    #[inline]
    pub fn render_target_image(&self, frame_label: FrameLabel) -> vk::Image
    {
        self.render_targets[frame_label as usize].image()
    }

    #[inline]
    pub fn render_target_image_view(&self, frame_label: FrameLabel) -> &Image2DView
    {
        self.render_targets[frame_label as usize].image_view()
    }

    pub fn color_image_bindless_handle(&self, bindless_mgr: &BindlessManager) -> shader::ImageHandle
    {
        bindless_mgr.get_image_handle(&self.color_image_view.uuid()).unwrap()
    }

    #[inline]
    pub fn color_image(&self) -> &Image2D
    {
        &self.color_image
    }

    #[inline]
    pub fn color_image_view(&self) -> &Image2DView
    {
        &self.color_image_view
    }

    pub fn render_target_image_bindless_handle(
        &self,
        bindless_mgr: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::ImageHandle
    {
        bindless_mgr.get_image_handle(&self.render_targets[frame_label as usize].image_view().uuid()).unwrap()
    }

    pub fn render_target_texture_bindless_handle(
        &self,
        bindless_mgr: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::TextureHandle
    {
        bindless_mgr.get_texture_handle(&self.render_target_bindless_keys[frame_label as usize]).unwrap()
    }
}
