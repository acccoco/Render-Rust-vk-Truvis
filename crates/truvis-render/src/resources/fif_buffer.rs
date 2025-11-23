use ash::vk;
use itertools::Itertools;

use truvis_gfx::{
    commands::barrier::GfxImageBarrier,
    gfx::Gfx,
    resources::handles::{ImageHandle, ImageViewHandle},
    sampler_manager::GfxSamplerDesc,
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
    accum_image: ImageHandle,
    accum_image_view: ImageViewHandle,

    depth_image: ImageHandle,
    depth_image_view: ImageViewHandle,

    /// 离屏渲染的结果，数量和 fif 相同
    off_screen_images: Vec<ImageHandle>,
    off_screen_image_views: Vec<ImageViewHandle>,
    off_screen_target_bindless_keys: Vec<String>,
}
impl FifBuffers {
    pub fn new(frame_settigns: &FrameSettings, bindless_manager: &mut BindlessManager, fif_count: usize) -> Self {
        let (color_image, color_image_view) = Self::create_color_image(frame_settigns);
        let (depth_image, depth_image_view) = Self::create_depth_image(frame_settigns);
        let (render_targets, render_target_views) = Self::create_render_targets(frame_settigns, fif_count);
        let render_target_bindless_keys = render_targets
            .iter()
            .enumerate()
            .map(|(i, _)| format!("render-target-{}", FrameLabel::from_usize(i)))
            .collect_vec();

        let fif_buffers = Self {
            accum_image: color_image,
            accum_image_view: color_image_view,
            depth_image,
            depth_image_view,
            off_screen_images: render_targets,
            off_screen_image_views: render_target_views,
            off_screen_target_bindless_keys: render_target_bindless_keys,
        };
        fif_buffers.register_bindless(bindless_manager);
        fif_buffers
    }

    /// 尺寸发生变化时，需要重新创建相关的资源
    pub fn rebuild(&mut self, frame_settings: &FrameSettings) {
        self.unregister_bindless();
        // Destroy old resources manually because ImageHandle is not RAII
        let mut rm = Gfx::get().resource_manager();
        rm.destroy_image_auto(self.accum_image);
        rm.destroy_image_auto(self.depth_image);
        for image in &self.off_screen_images {
            rm.destroy_image_auto(*image);
        }

        *self = Self::new(frame_settings, &mut FrameContext::bindless_manager_mut(), FrameContext::get().fif_count());
    }

    fn register_bindless(&self, bindless_manager: &mut BindlessManager) {
        bindless_manager.register_image(self.accum_image_view);

        let sampler = Gfx::get().sampler_manager().get_sampler(&GfxSamplerDesc::default());

        for (view, key) in self.off_screen_image_views.iter().zip(self.off_screen_target_bindless_keys.iter()) {
            bindless_manager.register_texture_handle(key.clone(), *view, sampler);
            bindless_manager.register_image(*view);
        }
    }

    fn unregister_bindless(&self) {
        let mut bindless_manager = FrameContext::bindless_manager_mut();

        bindless_manager.unregister_image2(self.accum_image_view);
        for (view, key) in self.off_screen_image_views.iter().zip(self.off_screen_target_bindless_keys.iter()) {
            bindless_manager.unregister_texture(key);
            bindless_manager.unregister_image2(*view);
        }
    }

    /// 创建 RayTracing 需要的 image
    fn create_color_image(frame_settings: &FrameSettings) -> (ImageHandle, ImageViewHandle) {
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(frame_settings.color_format)
            .extent(vk::Extent3D {
                width: frame_settings.frame_extent.width,
                height: frame_settings.frame_extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image_handle = Gfx::get().resource_manager().create_image(&create_info, &alloc_info, "fif-buffer-color");

        let image_view_handle = Gfx::get().resource_manager().get_image(image_handle).unwrap().default_view;
        let vk_image = Gfx::get().resource_manager().get_image(image_handle).unwrap().image;

        // layout transfer
        Gfx::get().one_time_exec(
            |cmd| {
                cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[GfxImageBarrier::new()
                        .image(vk_image)
                        .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                        .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                        .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)],
                );
            },
            "transfer-fif-buffer-color-image-layout",
        );

        (image_handle, image_view_handle)
    }

    fn create_depth_image(frame_settings: &FrameSettings) -> (ImageHandle, ImageViewHandle) {
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(frame_settings.depth_format)
            .extent(vk::Extent3D {
                width: frame_settings.frame_extent.width,
                height: frame_settings.frame_extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let image_handle = Gfx::get().resource_manager().create_image(&create_info, &alloc_info, "fif-buffer-depth");

        let image_view_handle = Gfx::get().resource_manager().get_image(image_handle).unwrap().default_view;

        (image_handle, image_view_handle)
    }

    fn create_render_targets(
        frame_settings: &FrameSettings,
        fif_count: usize,
    ) -> (Vec<ImageHandle>, Vec<ImageViewHandle>) {
        let mut images = Vec::with_capacity(fif_count);
        let mut views = Vec::with_capacity(fif_count);

        for i in 0..fif_count {
            let name = format!("render-target-{}", FrameLabel::from_usize(i));

            let create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(frame_settings.color_format)
                .extent(vk::Extent3D {
                    width: frame_settings.frame_extent.width,
                    height: frame_settings.frame_extent.height,
                    depth: 1,
                })
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(
                    vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::SAMPLED
                        | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                )
                .initial_layout(vk::ImageLayout::UNDEFINED);

            let alloc_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            };

            let image_handle = Gfx::get().resource_manager().create_image(&create_info, &alloc_info, &name);

            let image_view_handle = Gfx::get().resource_manager().get_image(image_handle).unwrap().default_view;

            images.push(image_handle);
            views.push(image_view_handle);
        }

        (images, views)
    }
}
// getter
impl FifBuffers {
    #[inline]
    pub fn depth_image_view(&self) -> ImageViewHandle {
        self.depth_image_view
    }

    #[inline]
    pub fn render_target_texture(&self, frame_label: FrameLabel) -> (ImageViewHandle, String) {
        (
            self.off_screen_image_views[frame_label as usize],
            self.off_screen_target_bindless_keys[frame_label as usize].clone(),
        )
    }

    #[inline]
    pub fn render_target_image(&self, frame_label: FrameLabel) -> vk::Image {
        let handle = self.off_screen_images[frame_label as usize];
        Gfx::get().resource_manager().get_image(handle).unwrap().image
    }

    #[inline]
    pub fn render_target_image_view(&self, frame_label: FrameLabel) -> ImageViewHandle {
        self.off_screen_image_views[frame_label as usize]
    }

    pub fn color_image_bindless_handle(&self, bindless_manager: &BindlessManager) -> shader::ImageHandle {
        bindless_manager.get_image_handle(self.accum_image_view).unwrap()
    }

    #[inline]
    pub fn color_image(&self) -> ImageHandle {
        self.accum_image
    }

    #[inline]
    pub fn color_image_view(&self) -> ImageViewHandle {
        self.accum_image_view
    }

    pub fn render_target_image_bindless_handle(
        &self,
        bindless_manager: &BindlessManager,
        frame_label: FrameLabel,
    ) -> shader::ImageHandle {
        bindless_manager.get_image_handle(self.off_screen_image_views[frame_label as usize]).unwrap()
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
        let mut rm = Gfx::get().resource_manager();
        rm.destroy_image_auto(self.accum_image);
        rm.destroy_image_auto(self.depth_image);
        for image in &self.off_screen_images {
            rm.destroy_image_auto(*image);
        }
    }
}
