use ash::vk;

use crate::{resource_type::shader::RhiShaderModule, rhi::Rhi};

pub struct RhiPipeline
{
    pub(crate) pipeline: vk::Pipeline,
    pub(crate) pipeline_layout: vk::PipelineLayout,
}

impl RhiPipeline {}


#[derive(Clone)]
pub struct RhiPipelineTemplate
{
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,

    pub color_formats: Vec<vk::Format>,
    pub depth_format: vk::Format,
    pub stencil_format: vk::Format,

    pub vertex_shader_path: Option<std::path::PathBuf>,
    pub fragment_shader_path: Option<std::path::PathBuf>,

    pub vertex_binding_desc: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_desec: Vec<vk::VertexInputAttributeDescription>,
    pub primitive_topology: vk::PrimitiveTopology,

    pub viewport: Option<vk::Viewport>,
    pub scissor: Option<vk::Rect2D>,

    pub rasterize_state_info: vk::PipelineRasterizationStateCreateInfo,

    pub msaa_sample: vk::SampleCountFlags,
    pub enable_sample_shading: bool,

    pub color_attach_blend_states: Vec<vk::PipelineColorBlendAttachmentState>,
    pub enable_logical_op: bool,

    pub depth_stencil_info: vk::PipelineDepthStencilStateCreateInfo,

    pub dynamic_states: Vec<vk::DynamicState>,
}

impl Default for RhiPipelineTemplate
{
    fn default() -> Self
    {
        Self {
            color_formats: vec![],

            // format = undefined 表示不使用这个 attachment
            depth_format: vk::Format::UNDEFINED,
            stencil_format: vk::Format::UNDEFINED,

            descriptor_set_layouts: vec![],

            vertex_shader_path: None,
            fragment_shader_path: None,

            vertex_binding_desc: vec![],
            vertex_attribute_desec: vec![],
            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,

            viewport: None,
            scissor: None,

            rasterize_state_info: vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::BACK)
                // FIXME 背面剔除，会涉及到 vulkan 的投影矩阵
                .front_face(vk::FrontFace::CLOCKWISE)
                .depth_bias_enable(false)
                .build(),

            msaa_sample: vk::SampleCountFlags::TYPE_1,
            enable_sample_shading: false,

            color_attach_blend_states: vec![],
            enable_logical_op: false,

            depth_stencil_info: vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build(),

            dynamic_states: vec![],
        }
    }
}

impl RhiPipelineTemplate
{
    pub fn create_pipeline(&self) -> RhiPipeline
    {
        let rhi = Rhi::instance();

        // dynamic rendering 需要的 framebuffer 信息
        let mut attach_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(&self.color_formats)
            .depth_attachment_format(self.depth_format)
            .stencil_attachment_format(self.stencil_format);

        let pipeline_layout = unsafe {
            let pipeline_layout_create_info =
                vk::PipelineLayoutCreateInfo::builder().set_layouts(&self.descriptor_set_layouts);
            unsafe { rhi.device().create_pipeline_layout(&pipeline_layout_create_info, None).unwrap() }
        };

        // vertex shader 和 fragment shader 是必须的，入口都是 main
        let vertex_shader_module = RhiShaderModule::new(self.vertex_shader_path.as_ref().unwrap());
        let fragment_shader_module = RhiShaderModule::new(self.fragment_shader_path.as_ref().unwrap());
        let shader_stages_info = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader_module.handle)
                .name(cstr::cstr!("main"))
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_module.handle)
                .name(cstr::cstr!("main"))
                .build(),
        ];

        // 顶点和 index
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&self.vertex_binding_desc)
            .vertex_attribute_descriptions(&self.vertex_attribute_desec);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(self.primitive_topology)
            .primitive_restart_enable(false);

        let viewport = self.viewport.as_ref().unwrap();
        let scissor = self.scissor.as_ref().unwrap();
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(std::slice::from_ref(viewport))
            .scissors(std::slice::from_ref(scissor));

        // MSAA 配置
        let msaa_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(self.enable_sample_shading)
            .rasterization_samples(self.msaa_sample);

        // 混合设置：需要为每个 color attachment 分别指定
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(self.enable_logical_op)
            .attachments(&self.color_attach_blend_states);

        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&self.dynamic_states);

        // =======================================
        // === 创建 pipeline

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages_info)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&self.rasterize_state_info)
            .multisample_state(&msaa_info)
            .color_blend_state(&color_blend_info)
            .depth_stencil_state(&self.depth_stencil_info)
            .layout(pipeline_layout)
            .dynamic_state(&dynamic_state_info)
            .push_next(&mut attach_info);

        let pipeline = unsafe {
            rhi.device()
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };

        vertex_shader_module.drop();
        fragment_shader_module.drop();

        RhiPipeline { pipeline, pipeline_layout }
    }
}
