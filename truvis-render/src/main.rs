use std::rc::Rc;

use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use shader_layout_macro::ShaderLayout;
use truvis_render::{
    framework::rendering::render_context::RenderContext,
    render::{App, AppCtx, AppInitInfo, Renderer},
};
use truvis_rhi::{
    core::image::{RhiImage2D, RhiImage2DView, RhiImageCreateInfo, RhiImageViewCreateInfo},
    render_core::Rhi,
    shader_cursor::ShaderCursorType,
};

fn main()
{
    Renderer::<VkApp>::run();
}

struct InitInfo
{
    width: u32,
    height: u32,
}

struct DepthStencil
{
    image: Rc<RhiImage2D>,
    view: Rc<RhiImage2DView>,
}

struct VkApp
{
    width: u32,
    height: u32,

    depth_stencil: DepthStencil,

    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    pipeline_cache: vk::PipelineCache,
}

#[derive(ShaderLayout)]
struct SceneShaderBindings
{
    #[binding = 0]
    #[stage = "VERTEX | FRAGMENT"]
    #[descriptor_type = "UNIFORM_BUFFER"]
    filed_0: ShaderCursorType,
    
    acc: vk::DescriptorType,

    #[binding = 1]
    #[stage = "FRAGMENT"]
    #[descriptor_type = "UNIFORM_BUFFER"]
    filed_1: ShaderCursorType,

    #[binding = 2]
    #[stage = "FRAGMENT"]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    filed_2: ShaderCursorType,

    #[binding = 3]
    #[stage = "FRAGMENT"]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    filed_3: ShaderCursorType,

    #[binding = 4]
    #[stage = "FRAGMENT"]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    filed_4: ShaderCursorType,
}


impl VkApp
{
    fn prepare_render_pass(rhi: &Rhi, render_ctx: &RenderContext) -> vk::RenderPass
    {
        // attachment
        let attachments = vec![
            // Color attachment
            vk::AttachmentDescription::default()
                .format(render_ctx.color_format())
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR),
            // Depth attachment
            vk::AttachmentDescription::default()
                .format(render_ctx.depth_format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL),
        ];

        let color_reference =
            vk::AttachmentReference::default().attachment(0).layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let depth_reference =
            vk::AttachmentReference::default().attachment(1).layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let subpass_description = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_reference))
            .depth_stencil_attachment(&depth_reference);

        let dependencies = vec![
            vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::MEMORY_READ)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dependency_flags(vk::DependencyFlags::BY_REGION),
            vk::SubpassDependency::default()
                .src_subpass(0)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::TOP_OF_PIPE) // FIXME 原文这里写的是 BOTTOM
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .dependency_flags(vk::DependencyFlags::BY_REGION),
        ];

        let render_pass_ci = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass_description))
            .dependencies(&dependencies);
        let render_pass = rhi.create_render_pass(&render_pass_ci, "main pass");

        render_pass
    }

    fn setup_depth_stencil(rhi: &Rhi, render_ctx: &RenderContext, init_info: &InitInfo) -> DepthStencil
    {
        // TODO 使用 vkmem

        // TODO 可以把这个 format 存下来
        let depth_format = render_ctx.depth_format;

        // depth image
        let depth_image = Rc::new(RhiImage2D::new(
            rhi,
            Rc::new(RhiImageCreateInfo::new_image_2d_info(
                vk::Extent2D {
                    width: init_info.width,
                    height: init_info.height,
                },
                depth_format,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            )),
            &vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            },
            "depth_image",
        ));

        let depth_image_view = Rc::new(RhiImage2DView::new(
            rhi,
            depth_image.clone(),
            RhiImageViewCreateInfo::new_image_view_2d_info(
                depth_format,
                vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL,
            ),
            "depth-image-view".to_string(),
        ));

        DepthStencil {
            image: depth_image,
            view: depth_image_view,
        }
    }

    fn setup_frame_buffer(
        rhi: &Rhi,
        render_context: &RenderContext,
        render_pass: vk::RenderPass,
        init_info: &InitInfo,
        depth_stencil: &DepthStencil,
    ) -> Vec<vk::Framebuffer>
    {
        let frame_buffers = render_context
            .render_swapchain
            .image_views
            .iter()
            .map(|image_view| {
                let attachments = [*image_view, depth_stencil.view.handle()];
                let frame_buffer_ci = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(init_info.width)
                    .height(init_info.height)
                    .layers(1);
                rhi.create_frame_buffer(&frame_buffer_ci, "frame buffer")
            })
            .collect_vec();

        frame_buffers
    }

    // TODO
    fn setup_desriptors(_rhi: &Rhi, _render_ctx: &RenderContext)
    {
        // scene descriptor sets: matrices and environment maps
        // 数量和 swapchain 的 image 保持一致
        {
            // let mut descriptor_sets = (0..render_ctx.render_swapchain.image_views.len())
            //     .map(|_| RhiDescriptorSet::new(rhi))
            //     .collect_vec();

            // FIXME
            // for mut descriptor_set in descriptor_sets {
            //     descriptor_set.write(vec![(0, RhiDescriptorUpdateInfo::Buffer(vk::DescriptorBufferInfo::default()))]);
            // }
        }

        // material descriptor sets

        // skybox descriptor sets
    }


    fn setup_pipelines() {}

    fn new(rhi: &Rhi, render_context: &mut RenderContext) -> Self
    {
        let init_info = InitInfo {
            width: 800,
            height: 800,
        };

        let render_pass = Self::prepare_render_pass(rhi, render_context);

        let pipeline_cache = rhi.create_pipeline_cache(&vk::PipelineCacheCreateInfo::default(), "pipeline cache");

        // TODO 考虑把这个挪到 render_context 里
        let depth_stencil = Self::setup_depth_stencil(rhi, render_context, &init_info);

        let frame_buffers = Self::setup_frame_buffer(rhi, render_context, render_pass, &init_info, &depth_stencil);

        // TODO 考虑将 lut 和 mipmap 的生成做成一个单独的 main()

        Self {
            width: init_info.width,
            height: init_info.height,

            depth_stencil,

            render_pass,
            framebuffers: frame_buffers,
            pipeline_cache,
        }
    }
}

impl App for VkApp
{
    fn update_ui(&mut self, _ui: &mut Ui)
    {
        todo!()
    }

    fn update(&mut self, _app_ctx: &mut AppCtx)
    {
        //
    }

    fn draw(&self, _app_ctx: &mut AppCtx)
    {
        todo!()
    }

    fn init(rhi: &Rhi, render_context: &mut RenderContext) -> Self
    {
        VkApp::new(rhi, render_context)
    }

    fn get_render_init_info() -> AppInitInfo
    {
        AppInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "Vk-glTF-PBR".to_string(),
            enable_validation: true,
        }
    }
}
