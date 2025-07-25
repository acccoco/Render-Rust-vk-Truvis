use crate::pipeline_settings::{FrameLabel, FrameSettings};
use crate::renderer::bindless::BindlessManager;
use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use std::rc::Rc;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::image::{RhiImage2D, RhiImage2DView, RhiImageCreateInfo, RhiImageViewCreateInfo};
use truvis_rhi::core::synchronize::RhiImageBarrier;
use truvis_rhi::core::texture::RhiTexture2D;
use truvis_rhi::rhi::Rhi;

/// 所有帧会用到的 buffers
pub struct FrameBuffers {
    color_image: Rc<RhiImage2D>,
    color_image_view: Rc<RhiImage2DView>,

    _depth_image: Rc<RhiImage2D>,
    depth_image_view: Rc<RhiImage2DView>,

    /// fif 每一帧的渲染结果
    render_targets: Vec<Rc<RhiTexture2D>>,
    render_target_bindless_keys: Vec<String>,
}

impl FrameBuffers {
    pub fn new(rhi: &Rhi, frame_settigns: &FrameSettings, bindless_mgr: &mut BindlessManager) -> Self {
        let (color_image, color_image_view) = Self::create_color_image(rhi, frame_settigns);
        let (depth_image, depth_image_view) = Self::create_depth_image(rhi, frame_settigns);
        let render_targets = Self::create_render_targets(rhi, frame_settigns);
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
        };
        framebuffers.register_bindless(bindless_mgr);
        framebuffers
    }

    pub fn rebuild(&mut self, rhi: &Rhi, frame_settings: &FrameSettings, bindless_mgr: &mut BindlessManager) {
        self.unregister_bindless(bindless_mgr);
        *self = Self::new(rhi, frame_settings, bindless_mgr);
    }

    fn register_bindless(&self, bindless_mgr: &mut BindlessManager) {
        bindless_mgr.register_image_shared(self.color_image_view.clone());

        for (render_target, key) in self.render_targets.iter().zip(self.render_target_bindless_keys.iter()) {
            bindless_mgr.register_texture_shared(key.clone(), render_target.clone());
            bindless_mgr.register_image_raw(render_target.image_view());
        }
    }

    fn unregister_bindless(&self, bindless_mgr: &mut BindlessManager) {
        bindless_mgr.unregister_image(&self.color_image_view.uuid());

        for (render_target, key) in self.render_targets.iter().zip(self.render_target_bindless_keys.iter()) {
            bindless_mgr.unregister_texture(key);
            bindless_mgr.unregister_image(&render_target.image_view().uuid())
        }
    }

    /// 创建 RayTracing 需要的 image
    fn create_color_image(rhi: &Rhi, frame_settings: &FrameSettings) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let color_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
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

        let color_image_view = Rc::new(RhiImage2DView::new(
            rhi,
            color_image.handle(),
            RhiImageViewCreateInfo::new_image_view_2d_info(frame_settings.color_format, vk::ImageAspectFlags::COLOR),
            "framebuffer-color",
        ));

        // layout transfer
        RhiCommandBuffer::one_time_exec(
            rhi,
            rhi.temp_graphics_command_pool.clone(),
            &rhi.graphics_queue,
            |cmd| {
                cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[RhiImageBarrier::new()
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

    fn create_depth_image(rhi: &Rhi, frame_settings: &FrameSettings) -> (Rc<RhiImage2D>, Rc<RhiImage2DView>) {
        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
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

        let depth_image_view = RhiImage2DView::new(
            rhi,
            depth_image.handle(),
            RhiImageViewCreateInfo::new_image_view_2d_info(frame_settings.depth_format, vk::ImageAspectFlags::DEPTH),
            "framebuffer-depth",
        );

        (depth_image, Rc::new(depth_image_view))
    }

    fn create_render_targets(rhi: &Rhi, frame_settings: &FrameSettings) -> Vec<Rc<RhiTexture2D>> {
        let create_texture = |fif_labe: FrameLabel| {
            let name = format!("render-target-{}", fif_labe);
            let color_image = Rc::new(RhiImage2D::new(
                rhi,
                Rc::new(RhiImageCreateInfo::new_image_2d_info(
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
            RhiTexture2D::new(rhi, color_image, &name)
        };

        (0..frame_settings.fif_num)
            .map(|fif_label| {
                let texture = create_texture(FrameLabel::from_usize(fif_label));
                Rc::new(texture)
            })
            .collect_vec()
    }
}

// getter
impl FrameBuffers {
    #[inline]
    pub fn depth_image_view(&self) -> &RhiImage2DView {
        &self.depth_image_view
    }

    #[inline]
    pub fn render_target_texture(&self, frame_label: FrameLabel) -> (&RhiTexture2D, String) {
        (
            &self.render_targets[frame_label as usize], //
            self.render_target_bindless_keys[frame_label as usize].clone(),
        )
    }

    #[inline]
    pub fn render_target_image(&self, frame_label: FrameLabel) -> vk::Image {
        self.render_targets[frame_label as usize].image()
    }

    #[inline]
    pub fn render_target_image_view(&self, frame_label: FrameLabel) -> &RhiImage2DView {
        self.render_targets[frame_label as usize].image_view()
    }

    pub fn color_image_bindless_handle(&self, bindless_mgr: &BindlessManager) -> shader::ImageHandle {
        bindless_mgr.get_image_handle(&self.color_image_view.uuid()).unwrap()
    }

    #[inline]
    pub fn color_image(&self) -> &RhiImage2D {
        &self.color_image
    }

    #[inline]
    pub fn color_image_view(&self) -> &RhiImage2DView {
        &self.color_image_view
    }

    pub fn render_target_image_bindless_handle(
        &self,
        bindless_mgr: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::ImageHandle {
        bindless_mgr.get_image_handle(&self.render_targets[frame_label as usize].image_view().uuid()).unwrap()
    }

    pub fn render_target_texture_bindless_handle(
        &self,
        bindless_mgr: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::TextureHandle {
        bindless_mgr.get_texture_handle(&self.render_target_bindless_keys[frame_label as usize]).unwrap()
    }
}
