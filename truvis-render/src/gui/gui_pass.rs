use crate::gui::mesh::ImGuiVertex;
use crate::gui::ui::Gui;
use crate::renderer::bindless::BindlessManager;
use crate::renderer::frame_context::FrameContext;
use crate::renderer::pipeline_settings::{FrameSettings, PipelineSettings};
use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use shader_binding::shader::TextureHandle;
use std::cell::RefCell;
use std::mem::offset_of;
use std::rc::Rc;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::create_named_array;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::graphics_pipeline::{RhiGraphicsPipeline, RhiGraphicsPipelineCreateInfo, RhiPipelineLayout};
use truvis_rhi::core::shader::RhiShaderStageInfo;
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
                stage: vk::ShaderStageFlags::FRAGMENT,
                entry_point: cstr::cstr!("psmain"),
                path: "shader/build/imgui/imgui.slang.spv",
            }
        ),
    ]
);

pub struct GuiPass {
    pipeline: RhiGraphicsPipeline,
    pipeline_layout: Rc<RhiPipelineLayout>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,
}
impl GuiPass {
    pub fn new(rhi: &Rhi, pipeline_settings: &PipelineSettings, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let pipeline_layout = Rc::new(RhiPipelineLayout::new(
            rhi.device.clone(),
            &[bindless_mgr.borrow().bindless_layout.handle()],
            &[vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<shader::imgui::PushConstant>() as u32,
            }],
            "uipass",
        ));

        let color_blend_attachments = vec![vk::PipelineColorBlendAttachmentState::default()
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

        let mut create_info = RhiGraphicsPipelineCreateInfo::default();
        create_info
            .shader_stages(ShaderStage::iter().map(|stage| *stage.value()).collect_vec())
            .vertex_attribute(ImGuiVertex::vertex_input_attributes())
            .vertex_binding(ImGuiVertex::vertex_input_bindings())
            .cull_mode(vk::CullModeFlags::NONE, vk::FrontFace::CLOCKWISE)
            .color_blend(color_blend_attachments, [0.0; 4])
            .depth_test(Some(vk::CompareOp::ALWAYS), false, false)
            .attach_info(vec![pipeline_settings.color_format], Some(pipeline_settings.depth_format), None);

        let pipeline = RhiGraphicsPipeline::new(rhi.device.clone(), &create_info, pipeline_layout.clone(), "uipass");

        Self {
            pipeline,
            pipeline_layout,
            bindless_mgr,
        }
    }

    pub fn draw(
        &self,
        rhi: &Rhi,
        render_ctx: &mut FrameContext,
        frame_settings: &FrameSettings,
        cmd: &RhiCommandBuffer,
        gui: &mut Gui,
    ) {
        let color_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(render_ctx.crt_present_image_view().handle())
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let depth_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(render_ctx.depth_view().handle())
            .image_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::LOAD)
            .store_op(vk::AttachmentStoreOp::STORE);
        let render_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(frame_settings.viewport_extent.into())
            .color_attachments(std::slice::from_ref(&color_attach_info))
            .depth_attachment(&depth_attach_info);

        let mesh;
        let draw_data;
        if let Some(r) = gui.imgui_render(rhi, cmd, render_ctx) {
            (mesh, draw_data) = r;
        } else {
            return;
        }

        let viewport = vk::Viewport {
            width: draw_data.framebuffer_scale[0] * draw_data.display_size[0],
            height: draw_data.framebuffer_scale[1] * draw_data.display_size[1],
            min_depth: 0.0,
            ..Default::default()
        };

        let frame_label = render_ctx.crt_frame_label();

        cmd.cmd_begin_rendering(&render_info);
        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());
        cmd.cmd_set_viewport(0, std::slice::from_ref(&viewport));
        let push_constant = shader::imgui::PushConstant {
            ortho: glam::Mat4::orthographic_rh(
                0.0,
                draw_data.display_size[0],
                0.0,
                draw_data.display_size[1],
                -1.0,
                1.0,
            )
            .into(),
            texture: TextureHandle { index: 0 },
            _padding_0: 0,
        };
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_layout.handle(),
            0,
            &[self.bindless_mgr.borrow().bindless_sets[*frame_label].handle()],
            None,
        );
        cmd.cmd_push_constants(
            self.pipeline_layout.handle(),
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            bytemuck::bytes_of(&push_constant),
        );
        cmd.cmd_bind_index_buffer(&mesh._index_buffer, 0, vk::IndexType::UINT16);
        cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&mesh.vertex_buffer), &[0]);

        let mut index_offset = 0;
        let mut vertex_offset = 0;
        // 缓存之前已经加载过的 texture
        let mut last_texture_id: Option<imgui::TextureId> = None;
        let clip_offset = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        let bindless_mgr = self.bindless_mgr.borrow();

        // 简而言之：对于每个 command，设置正确的 vertex, index, texture, scissor 即可
        for draw_list in draw_data.draw_lists() {
            for command in draw_list.commands() {
                match command {
                    imgui::DrawCmd::Elements {
                        count,
                        cmd_params:
                            imgui::DrawCmdParams {
                                clip_rect,
                                texture_id, // 当前绘制命令用到的 texture，这个 id 是 app 决定的
                                vtx_offset,
                                idx_offset,
                            },
                    } => {
                        let clip_x = (clip_rect[0] - clip_offset[0]) * clip_scale[0];
                        let clip_y = (clip_rect[1] - clip_offset[1]) * clip_scale[1];
                        let clip_w = (clip_rect[2] - clip_offset[0]) * clip_scale[0] - clip_x;
                        let clip_h = (clip_rect[3] - clip_offset[1]) * clip_scale[1] - clip_y;

                        let scissors = [vk::Rect2D {
                            offset: vk::Offset2D {
                                x: (clip_x as i32).max(0),
                                y: (clip_y as i32).max(0),
                            },
                            extent: vk::Extent2D {
                                width: clip_w as _,
                                height: clip_h as _,
                            },
                        }];
                        cmd.cmd_set_scissor(0, &scissors);

                        // 加载 texture，如果和上一个 command 使用的 texture 不是同一个，则需要重新加载
                        if Some(texture_id) != last_texture_id {
                            let texture_key = Gui::get_texture_key(texture_id);
                            let texture_handle = bindless_mgr.get_texture_idx(&texture_key).unwrap();
                            cmd.cmd_push_constants(
                                self.pipeline_layout.handle(),
                                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                                offset_of!(shader::imgui::PushConstant, texture) as u32,
                                bytemuck::bytes_of(&texture_handle),
                            );
                            last_texture_id = Some(texture_id);
                        }

                        cmd.draw_indexed(
                            count as u32,
                            index_offset + idx_offset as u32,
                            1,
                            0,
                            vertex_offset + vtx_offset as i32,
                        );
                    }
                    imgui::DrawCmd::ResetRenderState => {
                        log::warn!("imgui reset render state");
                    }
                    imgui::DrawCmd::RawCallback { .. } => {
                        log::warn!("imgui raw callback");
                    }
                }
            }

            index_offset += draw_list.idx_buffer().len() as u32;
            vertex_offset += draw_list.vtx_buffer().len() as i32;
        }
        cmd.end_rendering();
    }
}
