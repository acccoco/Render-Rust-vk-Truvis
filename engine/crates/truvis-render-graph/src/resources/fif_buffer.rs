use ash::vk;
use itertools::Itertools;
use slotmap::Key;
use truvis_gfx::resources::image_view::GfxImageViewDesc;
use truvis_gfx::{
    commands::barrier::GfxImageBarrier,
    gfx::Gfx,
    resources::image::{GfxImage, GfxImageCreateInfo},
};
use truvis_render_interface::bindless_manager::BindlessManager;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::gfx_resource_manager::GfxResourceManager;
use truvis_render_interface::handles::{GfxImageHandle, GfxImageViewHandle};
use truvis_render_interface::pipeline_settings::{FrameLabel, FrameSettings};

// TODO FifBuffers 放到 app 里面去，由 App 进行管理
/// 所有帧会用到的 buffers
pub struct FifBuffers {
    /// RT 计算的累积结果
    pub accum_image: GfxImageHandle,
    pub accum_image_view: GfxImageViewHandle,
    accum_format: vk::Format,
    accum_extent: vk::Extent2D,

    pub depth_image: GfxImageHandle,
    pub depth_image_view: GfxImageViewHandle,
    depth_format: vk::Format,
    depth_extent: vk::Extent2D,

    /// 离屏渲染的结果，数量和 fif 相同
    pub off_screen_target_image_handles: [GfxImageHandle; FrameCounter::fif_count()],
    pub off_screen_target_view_handles: [GfxImageViewHandle; FrameCounter::fif_count()],
    render_target_format: vk::Format,
    render_target_extent: vk::Extent2D,
}
// new & init
impl FifBuffers {
    pub fn new(
        frame_settigns: &FrameSettings,
        bindless_manager: &mut BindlessManager,
        gfx_resource_manager: &mut GfxResourceManager,
        frame_counter: &FrameCounter,
    ) -> Self {
        let accum_format = frame_settigns.color_format;
        let accum_extent = frame_settigns.frame_extent;
        let (color_image, color_image_view) =
            Self::create_color_image(gfx_resource_manager, accum_format, accum_extent, frame_counter);

        let depth_format = frame_settigns.depth_format;
        let depth_extent = frame_settigns.frame_extent;
        let (depth_image, depth_image_view) =
            Self::create_depth_image(gfx_resource_manager, depth_format, depth_extent, frame_counter);

        let render_target_format = frame_settigns.color_format;
        let render_target_extent = frame_settigns.frame_extent;
        let (render_target_image_handles, render_target_image_view_handles) = Self::create_render_targets(
            gfx_resource_manager,
            render_target_format,
            render_target_extent,
            frame_counter,
        );

        let fif_buffers = Self {
            accum_image: color_image,
            accum_image_view: color_image_view,
            accum_format,
            accum_extent,

            depth_image,
            depth_image_view,
            depth_extent,
            depth_format,

            off_screen_target_image_handles: render_target_image_handles,
            off_screen_target_view_handles: render_target_image_view_handles,
            render_target_format,
            render_target_extent,
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
        for render_target in &self.off_screen_target_view_handles {
            bindless_manager.register_uav(*render_target);
            bindless_manager.register_srv(*render_target);
        }
    }

    fn unregister_bindless(&self, bindless_manager: &mut BindlessManager) {
        bindless_manager.unregister_uav(self.accum_image_view);
        for render_target in &self.off_screen_target_view_handles {
            bindless_manager.unregister_uav(*render_target);
            bindless_manager.unregister_srv(*render_target);
        }
    }

    /// 创建 RayTracing 需要的 image
    fn create_color_image(
        gfx_resource_manager: &mut GfxResourceManager,
        format: vk::Format,
        extent: vk::Extent2D,
        frame_counter: &FrameCounter,
    ) -> (GfxImageHandle, GfxImageViewHandle) {
        let color_image_create_info = GfxImageCreateInfo::new_image_2d_info(
            extent,
            format,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED,
        );

        let color_image = GfxImage::new(
            &color_image_create_info,
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            &format!("fif-buffer-color-{}", frame_counter.frame_id()),
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
        let color_image_view_handle = gfx_resource_manager.get_or_create_image_view(
            color_image_handle,
            GfxImageViewDesc::new_2d(format, vk::ImageAspectFlags::COLOR),
            format!("fif-buffer-color-{}", frame_counter.frame_id()),
        );

        (color_image_handle, color_image_view_handle)
    }

    fn create_depth_image(
        gfx_resource_manager: &mut GfxResourceManager,
        format: vk::Format,
        extent: vk::Extent2D,
        frame_counter: &FrameCounter,
    ) -> (GfxImageHandle, GfxImageViewHandle) {
        let depth_image_create_info =
            GfxImageCreateInfo::new_image_2d_info(extent, format, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT);
        let depth_image = GfxImage::new(
            &depth_image_create_info,
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            &format!("fif-buffer-depth-{}", frame_counter.frame_id()),
        );
        let depth_image_handle = gfx_resource_manager.register_image(depth_image);
        let depth_image_view_handle = gfx_resource_manager.get_or_create_image_view(
            depth_image_handle,
            GfxImageViewDesc::new_2d(format, vk::ImageAspectFlags::DEPTH),
            format!("fif-buffer-depth-{}", frame_counter.frame_id()),
        );

        (depth_image_handle, depth_image_view_handle)
    }

    fn create_render_targets(
        gfx_resource_manager: &mut GfxResourceManager,
        format: vk::Format,
        extent: vk::Extent2D,
        frame_counter: &FrameCounter,
    ) -> ([GfxImageHandle; FrameCounter::fif_count()], [GfxImageViewHandle; FrameCounter::fif_count()]) {
        let create_one_target = |fif_labe: FrameLabel| {
            let name = format!("render-target-{}-{}", fif_labe, frame_counter.frame_id());

            let image_create_info = GfxImageCreateInfo::new_image_2d_info(
                extent,
                format,
                vk::ImageUsageFlags::STORAGE
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            );

            GfxImage::new(
                &image_create_info,
                &vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    ..Default::default()
                },
                &name,
            )
        };
        let images = FrameCounter::frame_labes().map(create_one_target);

        // 将 layout 设置为 general
        Gfx::get().one_time_exec(
            |cmd| {
                let image_barriers = images
                    .iter()
                    .map(|image| {
                        GfxImageBarrier::default()
                            .image(image.handle())
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

        let image_handles = images.map(|image| gfx_resource_manager.register_image(image));
        let image_view_handles = FrameCounter::frame_labes().map(|frame_label| {
            gfx_resource_manager.get_or_create_image_view(
                image_handles[*frame_label],
                GfxImageViewDesc::new_2d(format, vk::ImageAspectFlags::COLOR),
                format!("render-target-{}-{}", frame_label, frame_counter.frame_id()),
            )
        });

        (image_handles, image_view_handles)
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

        // 只需销毁 image，view 会跟随销毁
        for render_target_image in std::mem::take(&mut self.off_screen_target_image_handles) {
            gfx_resource_manager.destroy_image_immediate(render_target_image);
        }

        // image view 无需销毁，只需要销毁 image 即可
        gfx_resource_manager.destroy_image_immediate(self.depth_image);
        gfx_resource_manager.destroy_image_immediate(self.accum_image);

        self.depth_image_view = GfxImageViewHandle::default();
        self.accum_image_view = GfxImageViewHandle::default();
        self.depth_image = GfxImageHandle::default();
        self.accum_image = GfxImageHandle::default();
    }
}
impl Drop for FifBuffers {
    fn drop(&mut self) {
        debug_assert!(self.off_screen_target_image_handles.iter().all(|target| target.is_null()));
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
    pub fn render_target_handle(&self, frame_label: FrameLabel) -> (GfxImageHandle, GfxImageViewHandle) {
        (
            self.off_screen_target_image_handles[frame_label as usize],
            self.off_screen_target_view_handles[frame_label as usize],
        )
    }

    #[inline]
    pub fn color_image_handle(&self) -> GfxImageHandle {
        self.accum_image
    }

    #[inline]
    pub fn color_image_view_handle(&self) -> GfxImageViewHandle {
        self.accum_image_view
    }

    #[inline]
    pub fn color_image_format(&self) -> vk::Format {
        self.accum_format
    }

    #[inline]
    pub fn color_image_extent(&self) -> vk::Extent2D {
        self.accum_extent
    }

    #[inline]
    pub fn render_target_format(&self) -> vk::Format {
        self.render_target_format
    }
}
