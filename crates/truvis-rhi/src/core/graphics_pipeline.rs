use crate::core::device::RhiDevice;
use crate::core::shader::{RhiShaderModule, RhiShaderStageInfo};
use ash::vk;
use itertools::Itertools;
use std::ffi::CStr;
use std::rc::Rc;

pub struct RhiGraphicsPipelineCreateInfo {
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,

    push_constant_ranges: Vec<vk::PushConstantRange>,

    /// dynamic render 需要的 framebuffer 信息
    color_attach_formats: Vec<vk::Format>,
    /// dynamic render 需要的 framebuffer 信息
    depth_attach_format: vk::Format,
    /// dynamic render 需要的 framebuffer 信息
    stencil_attach_format: vk::Format,

    shader_stages: Vec<RhiShaderStageInfo>,

    vertex_binding_desc: Vec<vk::VertexInputBindingDescription>,
    vertex_attribute_desec: Vec<vk::VertexInputAttributeDescription>,

    primitive_topology: vk::PrimitiveTopology,

    // FIXME
    rasterize_state_info: vk::PipelineRasterizationStateCreateInfo<'static>,

    msaa_sample: vk::SampleCountFlags,
    enable_sample_shading: bool,

    color_attach_blend_states: Vec<vk::PipelineColorBlendAttachmentState>,
    enable_logical_op: bool,

    // FIXME
    depth_stencil_info: vk::PipelineDepthStencilStateCreateInfo<'static>,

    dynamic_states: Vec<vk::DynamicState>,
}

impl Default for RhiGraphicsPipelineCreateInfo {
    fn default() -> Self {
        Self {
            color_attach_formats: vec![],

            // format = undefined 表示不使用这个 attachment
            depth_attach_format: vk::Format::UNDEFINED,
            stencil_attach_format: vk::Format::UNDEFINED,

            descriptor_set_layouts: vec![],
            shader_stages: vec![],

            vertex_binding_desc: vec![],
            vertex_attribute_desec: vec![],

            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,

            rasterize_state_info: vk::PipelineRasterizationStateCreateInfo::default()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::BACK)
                // 按照 OpenGL 的传统，将 CCW 视为 front face
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false),
            msaa_sample: vk::SampleCountFlags::TYPE_1,
            enable_sample_shading: false,

            color_attach_blend_states: vec![],
            enable_logical_op: false,

            depth_stencil_info: vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false),
            dynamic_states: vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR],
            push_constant_ranges: vec![],
        }
    }
}

impl RhiGraphicsPipelineCreateInfo {
    /// builder
    #[inline]
    pub fn attach_info(
        &mut self,
        color_attach_formats: Vec<vk::Format>,
        depth_format: Option<vk::Format>,
        stencil_format: Option<vk::Format>,
    ) -> &mut Self {
        self.color_attach_formats = color_attach_formats;
        self.depth_attach_format = depth_format.unwrap_or(vk::Format::UNDEFINED);
        self.stencil_attach_format = stencil_format.unwrap_or(vk::Format::UNDEFINED);

        self
    }

    /// builder
    #[inline]
    pub fn vertex_shader_stage(&mut self, path: &'static str, entry_point: &'static CStr) -> &mut Self {
        self.shader_stages.push(RhiShaderStageInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            entry_point,
            path,
        });
        self
    }

    /// builder
    #[inline]
    pub fn fragment_shader_stage(&mut self, path: &'static str, entry_point: &'static CStr) -> &mut Self {
        self.shader_stages.push(RhiShaderStageInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            entry_point,
            path,
        });
        self
    }

    /// builder
    #[inline]
    pub fn vertex_binding(&mut self, bindings: Vec<vk::VertexInputBindingDescription>) -> &mut Self {
        self.vertex_binding_desc = bindings;
        self
    }

    /// builder
    #[inline]
    pub fn vertex_attribute(&mut self, attributes: Vec<vk::VertexInputAttributeDescription>) -> &mut Self {
        self.vertex_attribute_desec = attributes;
        self
    }

    /// builder
    #[inline]
    pub fn color_blend_attach_states(&mut self, states: Vec<vk::PipelineColorBlendAttachmentState>) -> &mut Self {
        self.color_attach_blend_states = states;
        self
    }

    /// builder
    #[inline]
    pub fn push_constant_ranges(&mut self, ranges: Vec<vk::PushConstantRange>) -> &mut Self {
        self.push_constant_ranges = ranges;
        self
    }

    /// builder
    #[inline]
    pub fn descriptor_set_layouts(&mut self, layouts: Vec<vk::DescriptorSetLayout>) -> &mut Self {
        self.descriptor_set_layouts = layouts;
        self
    }
}

pub struct RhiGraphicsPipeline {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    device: Rc<RhiDevice>,
}

impl Drop for RhiGraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            log::info!("Destroying RhiGraphicsPipeline");
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

impl RhiGraphicsPipeline {
    pub fn new(device: Rc<RhiDevice>, create_info: &RhiGraphicsPipelineCreateInfo, debug_name: &str) -> Self {
        // dynamic rendering 需要的 framebuffer 信息
        let mut attach_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&create_info.color_attach_formats)
            .depth_attachment_format(create_info.depth_attach_format)
            .stencil_attachment_format(create_info.stencil_attach_format);

        let pipeline_layout = {
            let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&create_info.descriptor_set_layouts)
                .push_constant_ranges(&create_info.push_constant_ranges);
            unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None).unwrap() }
        };
        device.debug_utils().set_object_debug_name(pipeline_layout, debug_name);

        let shader_modules = create_info
            .shader_stages
            .iter()
            .map(|stage| RhiShaderModule::new(device.clone(), stage.path()))
            .collect_vec();
        let shader_stages_info = create_info
            .shader_stages
            .iter()
            .zip(shader_modules.iter())
            .map(|(stage, module)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(stage.stage)
                    .module(module.handle())
                    .name(stage.entry_point)
            })
            .collect_vec();

        // 顶点和 index
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&create_info.vertex_binding_desc)
            .vertex_attribute_descriptions(&create_info.vertex_attribute_desec);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(create_info.primitive_topology)
            .primitive_restart_enable(false);

        // viewport 和 scissor 具体值由 dynamic 决定，但是数量由该 create info 决定
        let viewport_info = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };

        // MSAA 配置
        let msaa_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(create_info.enable_sample_shading)
            .rasterization_samples(create_info.msaa_sample);

        // 混合设置：需要为每个 color attachment 分别指定
        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(create_info.enable_logical_op)
            .attachments(&create_info.color_attach_blend_states);

        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&create_info.dynamic_states);

        // =======================================
        // === 创建 pipeline

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages_info)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&create_info.rasterize_state_info)
            .multisample_state(&msaa_info)
            .color_blend_state(&color_blend_info)
            .depth_stencil_state(&create_info.depth_stencil_info)
            .layout(pipeline_layout)
            .dynamic_state(&dynamic_state_info)
            .push_next(&mut attach_info);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };
        device.debug_utils().set_object_debug_name(pipeline, debug_name);

        shader_modules.into_iter().for_each(|module| {
            module.destroy();
        });

        RhiGraphicsPipeline {
            pipeline,
            pipeline_layout,
            device,
        }
    }

    #[inline]
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    #[inline]
    pub fn layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}
