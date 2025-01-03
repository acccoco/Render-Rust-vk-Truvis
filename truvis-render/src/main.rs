use ash::vk;
use truvis_render::{
    framework::{rendering::render_context::RenderContext, rhi::Rhi},
    render::{RenderInitInfo, Timer},
    run::App,
};

fn main() {}


struct VkApp {}

impl VkApp
{
    fn prepare_render_pass(rhi: &Rhi) -> vk::RenderPass
    {
        // attachment
        let attachments = vec![
            // Color attachment
            vk::AttachmentDescription::builder()
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build(),
            // Depth attachment
            vk::AttachmentDescription::builder()
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .build(),
        ];

        let color_reference =
            vk::AttachmentReference::builder().attachment(0).layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let depth_reference =
            vk::AttachmentReference::builder().attachment(1).layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let subpass_description = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_reference))
            .depth_stencil_attachment(&depth_reference);

        let dependencies = vec![
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::MEMORY_READ)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build(),
            vk::SubpassDependency::builder()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::TOP_OF_PIPE) // FIXME 原文这里写的是 BOTTOM
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build(),
        ];

        let render_pass_ci = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass_description))
            .dependencies(&dependencies)
            .build();
        let render_pass = rhi.create_render_pass(&render_pass_ci, "main pass");

        render_pass
    }

    fn setup_frame_buffer(rhi: &Rhi)
    {
        // depth image
        let image = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(rhi.get_depth_format()) // TODO 可以把这个 format 存下来
            .extent(vk::Extent3D {
                width: 800,
                height: 800,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .flags(vk::ImageCreateFlags::empty());
    }
}

impl App for VkApp
{
    fn new(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self
    {
        unimplemented!()
    }

    fn init_info() -> RenderInitInfo
    {
        unimplemented!()
    }

    fn get_init_info(&self) -> RenderInitInfo
    {
        RenderInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "Vk-glTF-PBR".to_string(),
        }
    }

    fn prepare(&mut self, rhi: &'static Rhi, render_context: &mut RenderContext)
    {
        let render_pass = Self::prepare_render_pass(rhi);

        // Pipeline cache
        let pipeline_cache_create_info = vk::PipelineCacheCreateInfo::default();
        let pipeline_cache = rhi.create_pipeline_cache(&pipeline_cache_create_info, "pipeline cache");

        todo!()
    }

    fn update(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        todo!()
    }
}
