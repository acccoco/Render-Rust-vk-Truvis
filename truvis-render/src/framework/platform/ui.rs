//! 参考 imgui-rs-vulkan-renderer

use std::{ffi::CString, mem};

use ash::vk;

use crate::framework::{core::shader::RhiShaderModule, rendering::render_context::RenderContext, rhi::Rhi};

pub struct UI {}


impl UI
{
    pub fn new(rhi: &'static Rhi, render_ctx: &RenderContext) -> Self
    {
        let descriptor_set_layout = Self::create_descriptor_set(&rhi.device.device);
        let pipeline_layout = Self::create_pipeline_layout(&rhi.device.device, descriptor_set_layout);
        let pipeline = Self::create_pipeline(rhi, render_ctx, pipeline_layout);
        
        let fonts_texture = {
            let fonts = imgui.fonts();
            let atlas_texture = fonts.build_rgba32_texture();

            Texture::from_rgba8(
                &device,
                queue,
                command_pool,
                &mut allocator,
                atlas_texture.width,
                atlas_texture.height,
                atlas_texture.data,
            )    
        };

        Self {}
    }

    // TODO refactor
    fn create_descriptor_set(device: &ash::Device) -> vk::DescriptorSetLayout
    {
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];

        let descriptor_set_create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        unsafe { device.create_descriptor_set_layout(&descriptor_set_create_info, None).unwrap() }
    }


    fn create_pipeline_layout(
        device: &ash::Device,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::PipelineLayout
    {
        let push_const_range = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: std::mem::size_of::<glam::Mat4>() as u32,
        }];

        let descriptor_set_layouts = [descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&descriptor_set_layouts)
            .push_constant_ranges(&push_const_range);
        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() };
        pipeline_layout
    }

    fn create_pipeline(
        rhi: &'static Rhi,
        render_ctx: &RenderContext,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline
    {
        let entry_point_name = CString::new("main").unwrap();

        let vertex_shader_source =
            std::include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../shader/imgui/shader.vert.spv"));
        let fragment_shader_source =
            std::include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../shader/imgui/shader.frag.spv"));

        let vert_shader_module = RhiShaderModule::new(rhi, std::path::Path::new("shader/imgui/shader.vert.spv"));
        let frag_shader_module = RhiShaderModule::new(rhi, std::path::Path::new("shader/imgui/shader.frag.spv"));

        let shader_states_infos = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader_module.handle)
                .name(&entry_point_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_shader_module.handle)
                .name(&entry_point_name),
        ];

        // 20 = R32G32 + R32G32 + R8G8B8A8
        let binding_desc = [vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(20)
            .input_rate(vk::VertexInputRate::VERTEX)];
        let attribute_desc = [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM)
                .offset(16),
        ];

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
                vk::ColorComponentFlags::R |
                    vk::ColorComponentFlags::G |
                    vk::ColorComponentFlags::B |
                    vk::ColorComponentFlags::A,
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
            .stages(&shader_states_infos)
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

        let color_attachment_formats = [render_ctx.color_format()];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(&color_attachment_formats)
            .depth_attachment_format(render_ctx.depth_format);

        let pipeline_info = pipeline_info.push_next(&mut rendering_info);

        let pipeline = unsafe {
            rhi.device
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), std::slice::from_ref(&pipeline_info), None)
                .unwrap()[0]
        };

        vert_shader_module.destroy();
        frag_shader_module.destroy();

        pipeline
    }
}
