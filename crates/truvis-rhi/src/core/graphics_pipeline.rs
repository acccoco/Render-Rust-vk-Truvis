use crate::core::debug_utils::RhiDebugType;
use crate::core::device::RhiDevice;
use crate::core::shader::{RhiShaderModule, RhiShaderStageInfo};
use ash::vk;
use itertools::Itertools;
use std::convert::identity;
use std::ffi::CStr;
use std::rc::Rc;

pub struct RhiGraphicsPipelineCreateInfo {
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

    rasterize_state_info: vk::PipelineRasterizationStateCreateInfo<'static>,

    msaa_sample: vk::SampleCountFlags,
    enable_sample_shading: bool,

    color_attach_blend_states: Vec<vk::PipelineColorBlendAttachmentState>,
    blend_info: vk::PipelineColorBlendStateCreateInfo<'static>,

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
            blend_info: vk::PipelineColorBlendStateCreateInfo::default()
                .logic_op_enable(false)
                .blend_constants([0.0, 0.0, 0.0, 0.0]),

            depth_stencil_info: vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(vk::CompareOp::LESS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false),
            dynamic_states: vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR],
        }
    }
}
// builder
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

    #[inline]
    pub fn shader_stages(&mut self, stages: Vec<RhiShaderStageInfo>) -> &mut Self {
        self.shader_stages = stages;
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

    /// 为每个 color attachment 指定 blend 操作
    #[inline]
    pub fn color_blend(
        &mut self,
        states: Vec<vk::PipelineColorBlendAttachmentState>,
        blend_constants: [f32; 4],
    ) -> &mut Self {
        self.color_attach_blend_states = states;
        self.blend_info.blend_constants = blend_constants;
        self.blend_info.logic_op_enable = vk::FALSE;
        self
    }

    /// logic op 和 blend op 是互斥的
    #[inline]
    pub fn blend_logic_op(&mut self, logic_op: vk::LogicOp) -> &mut Self {
        self.blend_info.logic_op = logic_op;
        self.blend_info.logic_op_enable = vk::TRUE;
        self
    }

    #[inline]
    pub fn cull_mode(&mut self, mode: vk::CullModeFlags, front_face: vk::FrontFace) -> &mut Self {
        self.rasterize_state_info.cull_mode = mode;
        self.rasterize_state_info.front_face = front_face;
        self
    }

    #[inline]
    pub fn depth_test(
        &mut self,
        depth_test_op: Option<vk::CompareOp>,
        depth_write: bool,
        depth_bounds_test: bool,
    ) -> &mut Self {
        self.depth_stencil_info.depth_test_enable = depth_test_op.map_or(vk::FALSE, |_| vk::TRUE);
        self.depth_stencil_info.depth_compare_op = depth_test_op.map_or(vk::CompareOp::NEVER, identity);
        self.depth_stencil_info.depth_write_enable = if depth_write { vk::TRUE } else { vk::FALSE };
        self.depth_stencil_info.depth_bounds_test_enable = if depth_bounds_test { vk::TRUE } else { vk::FALSE };
        self
    }

    #[inline]
    pub fn stencil_test(&mut self, enable: bool) -> &mut Self {
        self.depth_stencil_info.stencil_test_enable = if enable { vk::TRUE } else { vk::FALSE };
        self
    }
}

pub struct RhiPipelineLayout {
    handle: vk::PipelineLayout,
    device: Rc<RhiDevice>,
}
impl RhiDebugType for RhiPipelineLayout {
    fn debug_type_name() -> &'static str {
        "RhiPipelineLayouer"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
impl Drop for RhiPipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline_layout(self.handle, None);
        }
    }
}
impl RhiPipelineLayout {
    pub fn new(
        device: Rc<RhiDevice>,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
        debug_name: impl AsRef<str>,
    ) -> Self {
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(descriptor_set_layouts)
            .push_constant_ranges(push_constant_ranges);
        let handle = unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None).unwrap() };
        let layout = RhiPipelineLayout {
            handle,
            device: device.clone(),
        };
        device.debug_utils().set_debug_name(&layout, debug_name);
        layout
    }

    #[inline]
    pub fn handle(&self) -> vk::PipelineLayout {
        self.handle
    }
}

pub struct RhiGraphicsPipeline {
    pipeline: vk::Pipeline,

    /// 因为多个 pipeline 可以使用同一个 pipeline layout，所以这里使用 Rc
    pipeline_layout: Rc<RhiPipelineLayout>,

    device: Rc<RhiDevice>,
}
impl RhiDebugType for RhiGraphicsPipeline {
    fn debug_type_name() -> &'static str {
        "RhiGraphicsPipeline"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.pipeline
    }
}
impl Drop for RhiGraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}
impl RhiGraphicsPipeline {
    pub fn new(
        device: Rc<RhiDevice>,
        create_info: &RhiGraphicsPipelineCreateInfo,
        pipeline_layout: Rc<RhiPipelineLayout>,
        debug_name: &str,
    ) -> Self {
        // dynamic rendering 需要的 framebuffer 信息
        let mut attach_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&create_info.color_attach_formats)
            .depth_attachment_format(create_info.depth_attach_format)
            .stencil_attachment_format(create_info.stencil_attach_format);

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
        let color_blend_info = create_info.blend_info.attachments(&create_info.color_attach_blend_states);

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
            .layout(pipeline_layout.handle)
            .dynamic_state(&dynamic_state_info)
            .push_next(&mut attach_info);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };
        let pipeline = RhiGraphicsPipeline {
            pipeline,
            pipeline_layout,
            device: device.clone(),
        };

        device.debug_utils().set_debug_name(&pipeline, debug_name);

        shader_modules.into_iter().for_each(|module| {
            module.destroy();
        });

        pipeline
    }

    #[inline]
    pub fn handle(&self) -> vk::Pipeline {
        self.pipeline
    }

    #[inline]
    pub fn layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout.handle
    }
}
