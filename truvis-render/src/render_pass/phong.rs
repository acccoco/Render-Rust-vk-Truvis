use crate::app::AppCtx;
use crate::frame_context::FrameContext;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_scene::GpuScene;
use ash::vk;
use model_manager::vertex::vertex_3d::VertexLayoutAos3D;
use model_manager::vertex::vertex_pnu::VertexLayoutAosPosNormalUv;
use model_manager::vertex::VertexLayout;
use shader_binding::shader;
use std::cell::RefCell;
use std::mem::offset_of;
use std::rc::Rc;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::graphics_pipeline::{RhiGraphicsPipeline, RhiGraphicsPipelineCreateInfo};
use truvis_rhi::rhi::Rhi;

pub struct SimpleMainPass {
    pipeline: RhiGraphicsPipeline,

    _bindless_manager: Rc<BindlessManager>,
}
impl SimpleMainPass {
    pub fn new(rhi: &Rhi, frame_context: &FrameContext, bindless_manager: Rc<BindlessManager>) -> Self {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong.vs.slang.spv".to_string(), "main".to_string());
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv".to_string(), "main".to_string());
        ci.push_constant_ranges(vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<shader::DrawData>() as u32)]);
        // ci.descriptor_set_layouts(vec![bindless_manager.bindless_layout.layout]);
        ci.attach_info(vec![frame_context.color_format()], Some(frame_context.depth_format()), None);
        ci.vertex_binding(VertexLayoutAosPosNormalUv::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAosPosNormalUv::vertex_input_attributes());
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        let simple_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-simple-pipe");

        Self {
            pipeline: simple_pipe,
            _bindless_manager: bindless_manager,
        }
    }

    pub fn bind(&self, cmd: &RhiCommandBuffer, viewport: &vk::Rect2D, push_constant: &shader::DrawData) {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: viewport.offset.x as f32,
                y: viewport.offset.y as f32 + viewport.extent.height as f32,
                width: viewport.extent.width as f32,
                height: -(viewport.extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_push_constants(
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constant),
        );
        // cmd.bind_descriptor_sets(
        //     vk::PipelineBindPoint::GRAPHICS,
        //     self.pipeline.pipeline_layout,
        //     0,
        //     &[self.bindless_manager.bindless_set.handle],
        //     &[0],
        // );
    }

    pub fn draw(&self, cmd: &RhiCommandBuffer, app_ctx: &mut AppCtx) {
        // cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline_simple.pipeline);

        let swapchain_extend = app_ctx.render_context.swapchain_extent();
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: 0.0,
                y: swapchain_extend.height as f32,
                width: swapchain_extend.width as f32,
                height: -(swapchain_extend.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: swapchain_extend,
            }],
        );

        // cmd.cmd_push_constants(
        //     self.pipeline_simple.pipeline_layout,
        //     vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        //     0,
        //     bytemuck::bytes_of(&self.push),
        // );
        //
        // // scene data
        // cmd.bind_descriptor_sets(
        //     vk::PipelineBindPoint::GRAPHICS,
        //     self.pipeline_simple.pipeline_layout,
        //     0,
        //     &[self.descriptor_sets[frame_id].scene_set.handle],
        //     &[0],
        // );
        //
        // // per mat
        // cmd.bind_descriptor_sets(
        //     vk::PipelineBindPoint::GRAPHICS,
        //     self.pipeline_simple.pipeline_layout,
        //     2,
        //     &[self.descriptor_sets[frame_id].material_set.handle],
        //     // TODO 只使用一个材质
        //     &[0],
        // );

        // index 和 vertex 暂且就用同一个
        // cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.cube.vertex_buffer), &[0]);
        // cmd.cmd_bind_index_buffer(&self.cube.index_buffer, 0, vk::IndexType::UINT32);
        //
        // for (mesh_idx, _) in self.mesh_ubo.iter().enumerate() {
        //     cmd.bind_descriptor_sets(
        //         vk::PipelineBindPoint::GRAPHICS,
        //         self.pipeline_simple.pipeline_layout,
        //         1,
        //         &[self.descriptor_sets[frame_id].mesh_set.handle],
        //         &[(self.mesh_ubo_offset_align * mesh_idx as u64) as u32],
        //     );
        //     cmd.draw_indexed(self.cube.index_cnt, 0, 1, 0, 0);
        //     // cmd.cmd_draw(VertexPosNormalUvAoS::shape_box().len() as u32, 1, 0, 0);
        // }
    }
}

pub struct Simple3DMainPass {
    pipeline: RhiGraphicsPipeline,
    bindless_manager: Rc<RefCell<BindlessManager>>,
}
impl Simple3DMainPass {
    pub fn new(rhi: &Rhi, frame_context: &FrameContext, bindless_manager: Rc<RefCell<BindlessManager>>) -> Self {
        let mut ci = RhiGraphicsPipelineCreateInfo::default();
        ci.vertex_shader_stage("shader/build/phong/phong3d.vs.slang.spv".to_string(), "main".to_string());
        ci.fragment_shader_stage("shader/build/phong/phong.ps.slang.spv".to_string(), "main".to_string());

        ci.vertex_binding(VertexLayoutAos3D::vertex_input_bindings());
        ci.vertex_attribute(VertexLayoutAos3D::vertex_input_attributes());

        ci.push_constant_ranges(vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<shader::DrawData>() as u32)]);
        ci.descriptor_set_layouts(vec![bindless_manager.borrow().bindless_layout.layout]);
        ci.attach_info(vec![frame_context.color_format()], Some(frame_context.depth_format()), None);
        ci.color_blend_attach_states(vec![vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)]);

        let d3_pipe = RhiGraphicsPipeline::new(rhi.device.clone(), &ci, "phong-d3-pipe");

        Self {
            pipeline: d3_pipe,
            bindless_manager,
        }
    }

    fn bind(&self, cmd: &RhiCommandBuffer, viewport: &vk::Rect2D, push_constant: &shader::DrawData, frame_idx: usize) {
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
        cmd.cmd_set_viewport(
            0,
            &[vk::Viewport {
                x: viewport.offset.x as f32,
                y: viewport.offset.y as f32 + viewport.extent.height as f32,
                width: viewport.extent.width as f32,
                height: -(viewport.extent.height as f32),
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );
        cmd.cmd_set_scissor(0, &[*viewport]);
        cmd.cmd_push_constants(
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(push_constant),
        );

        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline.pipeline_layout,
            0,
            &[self.bindless_manager.borrow().bindless_sets[frame_idx].handle],
            &[],
        );
    }

    pub fn draw(
        &self,
        cmd: &RhiCommandBuffer,
        app_ctx: &AppCtx,
        push_constant: &shader::DrawData,
        scene_data: &GpuScene,
        frame_idx: usize,
    ) {
        self.bind(cmd, &app_ctx.render_context.swapchain_extent().into(), push_constant, frame_idx);

        scene_data.draw(cmd, &mut |ins_idx| {
            cmd.cmd_push_constants(
                self.pipeline.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset_of!(shader::DrawData, instance_id) as u32,
                bytemuck::bytes_of(&ins_idx),
            );
        });
    }
}
