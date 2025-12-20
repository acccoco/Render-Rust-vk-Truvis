use ash::vk;
use itertools::Itertools;
use slotmap::Key;
use truvis_gfx::resources::image_view::GfxImageViewDesc;
use truvis_gfx::{
    commands::barrier::GfxImageBarrier,
    gfx::Gfx,
    resources::image::{GfxImage, GfxImageCreateInfo},
};
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_render_base::frame_counter::FrameCounter;
use truvis_render_base::pipeline_settings::{FrameLabel, FrameSettings};
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_resource::handles::{GfxImageHandle, GfxImageViewHandle, GfxTextureHandle};
use truvis_resource::texture::GfxTexture;

/// 所有帧会用到的 buffers
pub struct FifBuffers {
    /// RT 计算的累积结果
    accum_image: GfxImageHandle,
    accum_image_view: GfxImageViewHandle,

    depth_image: GfxImageHandle,
    depth_image_view: GfxImageViewHandle,

    /// 离屏渲染的结果，数量和 fif 相同
    off_screen_targets: [GfxTextureHandle; FrameCounter::fif_count()],
}
// new & init
impl FifBuffers {
    pub fn new(
        frame_settigns: &FrameSettings,
        bindless_manager: &mut BindlessManager,
        gfx_resource_manager: &mut GfxResourceManager,
        frame_counter: &FrameCounter,
    ) -> Self {
        let (color_image, color_image_view) =
            Self::create_color_image(gfx_resource_manager, frame_settigns, frame_counter);
        let (depth_image, depth_image_view) =
            Self::create_depth_image(gfx_resource_manager, frame_settigns, frame_counter);
        let render_targets = Self::create_render_targets(gfx_resource_manager, frame_settigns, frame_counter);

        let fif_buffers = Self {
            accum_image: color_image,
            accum_image_view: color_image_view,
            depth_image,
            depth_image_view,
            off_screen_targets: render_targets,
        };
        fif_buffers.register_bindless(bindless_manager);
        fif_buffers
    }

    /// 尺寸发生变化时，需要重新创建相关的资源
    pub fn rebuild(
        &mut self,
        bindless_manager: &mut BindlessManager,
        gfx_resource_manager: &mut GfxResourceManager,
        frame_settings: &FrameSettings,
        frame_counter: &FrameCounter,
    ) {
        self.destroy_mut(bindless_manager, gfx_resource_manager);
        *self = Self::new(frame_settings, bindless_manager, gfx_resource_manager, frame_counter);
    }

    fn register_bindless(&self, bindless_manager: &mut BindlessManager) {
        bindless_manager.register_uav(self.accum_image_view);
        for render_target in &self.off_screen_targets {
            bindless_manager.register_srv_with_texture(*render_target);
            bindless_manager.register_uav_with_texture(*render_target);
        }
    }

    fn unregister_bindless(&self, bindless_manager: &mut BindlessManager) {
        bindless_manager.unregister_uav(self.accum_image_view);
        for render_target in &self.off_screen_targets {
            bindless_manager.unregister_srv_with_texture(*render_target);
            bindless_manager.unregister_uav_with_texture(*render_target);
        }
    }

    /// 创建 RayTracing 需要的 image
    fn create_color_image(
        gfx_resource_manager: &mut GfxResourceManager,
        frame_settings: &FrameSettings,
        frame_counter: &FrameCounter,
    ) -> (GfxImageHandle, GfxImageViewHandle) {
        let color_image_create_info = GfxImageCreateInfo::new_image_2d_info(
            frame_settings.frame_extent,
            frame_settings.color_format,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
        );

        let color_image = GfxImage::new(
            &color_image_create_info,
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            &format!("fif-buffer-color-{}", frame_counter.frame_id),
        );

        // 将 layout 设置为 general
        Gfx::get().one_time_exec(
            |cmd| {
                cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[GfxImageBarrier::new()
                        .image(color_image.handle())
                        .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                        .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                        .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)],
                );
            },
            "transfer-fif-buffer-color-image-layout",
        );

        let color_image_handle = gfx_resource_manager.register_image(color_image);
        let color_image_view_handle = gfx_resource_manager.try_create_image_view(
            color_image_handle,
            GfxImageViewDesc::new_2d(frame_settings.color_format, vk::ImageAspectFlags::COLOR),
            format!("fif-buffer-color-{}", frame_counter.frame_id),
        );

        (color_image_handle, color_image_view_handle)
    }

    fn create_depth_image(
        gfx_resource_manager: &mut GfxResourceManager,
        frame_settings: &FrameSettings,
        frame_counter: &FrameCounter,
    ) -> (GfxImageHandle, GfxImageViewHandle) {
        let depth_image_create_info = GfxImageCreateInfo::new_image_2d_info(
            frame_settings.frame_extent,
            frame_settings.depth_format,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        );
        let depth_image = GfxImage::new(
            &depth_image_create_info,
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            &format!("fif-buffer-depth-{}", frame_counter.frame_id),
        );
        let depth_image_handle = gfx_resource_manager.register_image(depth_image);
        let depth_image_view_handle = gfx_resource_manager.try_create_image_view(
            depth_image_handle,
            GfxImageViewDesc::new_2d(frame_settings.depth_format, vk::ImageAspectFlags::DEPTH),
            format!("fif-buffer-depth-{}", frame_counter.frame_id),
        );

        (depth_image_handle, depth_image_view_handle)
    }

    fn create_render_targets(
        gfx_resource_manager: &mut GfxResourceManager,
        frame_settings: &FrameSettings,
        frame_counter: &FrameCounter,
    ) -> [GfxTextureHandle; FrameCounter::fif_count()] {
        let create_texture = |fif_labe: FrameLabel| {
            let name = format!("render-target-{}-{}", fif_labe, frame_counter.frame_id);

            let image_create_info = GfxImageCreateInfo::new_image_2d_info(
                frame_settings.frame_extent,
                frame_settings.color_format,
                vk::ImageUsageFlags::STORAGE
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            );

            let color_image = GfxImage::new(
                &image_create_info,
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    ..Default::default()
                },
                &name,
            );
            GfxTexture::new(color_image, &name)
        };
        let textures = FrameCounter::frame_labes().map(create_texture);

        // 将 layout 设置为 general
        Gfx::get().one_time_exec(
            |cmd| {
                let image_barriers = textures
                    .iter()
                    .map(|texture| {
                        GfxImageBarrier::default()
                            .image(texture.image().handle())
                            .src_mask(vk::PipelineStageFlags2::TOP_OF_PIPE, vk::AccessFlags2::empty())
                            .dst_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE, vk::AccessFlags2::empty())
                            .layout_transfer(vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL)
                            .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                    })
                    .collect_vec();

                cmd.image_memory_barrier(vk::DependencyFlags::empty(), &image_barriers);
            },
            "transfer-fif-buffer-render-target-layout",
        );

        textures.map(|texture| gfx_resource_manager.register_texture(texture))
    }
}
// destroy
impl FifBuffers {
    pub fn destroy_mut(
        &mut self,
        bindless_manager: &mut BindlessManager,
        gfx_resource_manager: &mut GfxResourceManager,
    ) {
        self.unregister_bindless(bindless_manager);

        for render_target in std::mem::take(&mut self.off_screen_targets) {
            gfx_resource_manager.destroy_texture_auto(render_target);
        }

        // image view 无需销毁，只需要销毁 image 即可
        gfx_resource_manager.destroy_image_auto(self.depth_image);
        gfx_resource_manager.destroy_image_auto(self.accum_image);

        self.depth_image_view = GfxImageViewHandle::default();
        self.accum_image_view = GfxImageViewHandle::default();
        self.depth_image = GfxImageHandle::default();
        self.accum_image = GfxImageHandle::default();
    }
}
impl Drop for FifBuffers {
    fn drop(&mut self) {
        debug_assert!(self.off_screen_targets.iter().all(|target| target.is_null()));
        debug_assert!(self.depth_image.is_null());
        debug_assert!(self.depth_image_view.is_null());
        debug_assert!(self.accum_image.is_null());
        debug_assert!(self.accum_image_view.is_null());
    }
}
// getter
impl FifBuffers {
    #[inline]
    pub fn depth_image_view_handle(&self) -> GfxImageViewHandle {
        self.depth_image_view
    }

    #[inline]
    pub fn render_target_texture_handle(&self, frame_label: FrameLabel) -> GfxTextureHandle {
        self.off_screen_targets[frame_label as usize]
    }

    #[inline]
    pub fn color_image_handle(&self) -> GfxImageHandle {
        self.accum_image
    }

    #[inline]
    pub fn color_image_view_handle(&self) -> GfxImageViewHandle {
        self.accum_image_view
    }
}
