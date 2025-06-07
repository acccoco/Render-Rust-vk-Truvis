use crate::gui::mesh::ImGuiVertex;
use crate::pipeline_settings::PipelineSettings;
use ash::vk;
use ash::vk::ShaderModule;
use itertools::Itertools;
use shader_layout_macro::ShaderLayout;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::create_named_array;
use truvis_rhi::core::descriptor::RhiDescriptorSetLayout;
use truvis_rhi::core::shader::{RhiShaderModule, RhiShaderStageInfo};
use truvis_rhi::rhi::Rhi;

create_named_array!(
    ShaderStage,
    SHADER_STAGES,
    RhiShaderStageInfo,
    [
        (
            Vertex,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                entry_point: cstr::cstr!("vsmain"),
                path: "shader/build/imgui/imgui.slang.spv",
            }
        ),
        (
            Fragment,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                entry_point: cstr::cstr!("psmain"),
                path: "shader/build/imgui/imgui.slang.spv",
            }
        ),
    ]
);

#[derive(ShaderLayout)]
struct UiShaderLayout {
    #[binding = 0]
    #[descriptor_type = "COMBINED_IMAGE_SAMPLER"]
    #[stage = "FRAGMENT"]
    _font: (),
}

pub struct GuiPass {}
impl GuiPass {
    fn new(rhi: &Rhi, pipeline_settings: &PipelineSettings) -> Self {
        let descriptor_set_layout = RhiDescriptorSetLayout::<UiShaderLayout>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "[uipass]descriptor-set-layout",
        );

        let pipeline_layout = Self::create_pipeline_layout(rhi.device.handle(), descriptor_set_layout.handle());
        let pipeline =
            Self::create_pipeline(rhi, pipeline_settings.color_format, pipeline_settings.depth_format, pipeline_layout);

        Self {}
    }

    fn create_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::PipelineLayout {
        let push_const_range = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: size_of::<glam::Mat4>() as u32,
        }];

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_const_range);

        unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() }
    }

    fn create_pipeline(
        rhi: &Rhi,
        color_format: vk::Format,
        depth_format: vk::Format,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline {
        let mut shader_modules = ShaderStage::iter()
            .map(|stage| RhiShaderModule::new(rhi.device.clone(), stage.value().path()))
            .collect_vec();
        let stage_infos = ShaderStage::iter()
            .zip(shader_modules.iter())
            .map(|(stage, shader_module)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(stage.value().stage)
                    .module(shader_module.handle())
                    .name(stage.value().entry_point)
            })
            .collect_vec();

        // 20 = R32G32 + R32G32 + R8G8B8A8
        let binding_desc = ImGuiVertex::vertex_input_bindings();
        let attribute_desc = ImGuiVertex::vertex_input_attributes();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_desc)
            .vertex_attribute_descriptions(&attribute_desc);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let viewports = [Default::default()];
        let scissors = [Default::default()];
        let viewport_info = vk::PipelineViewportStateCreateInfo::default().viewports(&viewports).scissors(&scissors);

        let multisampling_info = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1) // fixme msaa 1
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)];
        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(false) // FIXME
            .depth_write_enable(false) // FIXME
            .depth_compare_op(vk::CompareOp::ALWAYS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let dynamic_states = [vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT];
        let dynamic_states_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stage_infos)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .rasterization_state(&rasterizer_info)
            .viewport_state(&viewport_info)
            .multisample_state(&multisampling_info)
            .color_blend_state(&color_blending_info)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .dynamic_state(&dynamic_states_info)
            .layout(pipeline_layout)
            .subpass(0);

        let color_attachment_formats = [color_format];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats)
            .depth_attachment_format(depth_format);

        let pipeline_info = pipeline_info.push_next(&mut rendering_info);

        let pipeline = unsafe {
            rhi.device
                .handle()
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };

        std::mem::take(&mut shader_modules).into_iter().for_each(|shader_module| {
            shader_module.destroy();
        });

        pipeline
    }
}
