use std::{cell::RefCell, mem::offset_of, rc::Rc};

use ash::vk;
use itertools::Itertools;
use shader_binding::{shader, shader::TextureHandle};
use truvis_crate_tools::{const_map, count_indexed_array, resource::TruvisPath};
use truvis_rhi::{
    commands::command_buffer::CommandBuffer,
    pipelines::{
        graphics_pipeline::{GraphicsPipeline, GraphicsPipelineCreateInfo, PipelineLayout},
        shader::ShaderStageInfo,
    },
    render_context::RenderContext,
};

use crate::{
    gui::{gui::Gui, mesh::ImGuiVertex},
    pipeline_settings::FrameLabel,
    renderer::bindless::BindlessManager,
};

const_map!(ShaderStage<ShaderStageInfo>: {
    Vertex: ShaderStageInfo {
        stage: vk::ShaderStageFlags::VERTEX,
        entry_point: cstr::cstr!("vsmain"),
        path: TruvisPath::shader_path("imgui/imgui.slang.spv"),
    },
    Fragment: ShaderStageInfo {
        stage: vk::ShaderStageFlags::FRAGMENT,
        entry_point: cstr::cstr!("psmain"),
        path: TruvisPath::shader_path("imgui/imgui.slang.spv"),
    },
});

pub struct GuiPass {
    pipeline: GraphicsPipeline,
    pipeline_layout: Rc<PipelineLayout>,
    bindless_mgr: Rc<RefCell<BindlessManager>>,
}

impl GuiPass {
    pub fn new(
        render_context: &RenderContext,
        bindless_mgr: Rc<RefCell<BindlessManager>>,
        color_format: vk::Format,
    ) -> Self {
        let pipeline_layout = Rc::new(PipelineLayout::new(
            render_context.device_functions(),
            &[bindless_mgr.borrow().bindless_descriptor_layout.handle()],
            &[vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<shader::imgui::PushConstant>() as u32,
            }],
            "uipass",
        ));

        let color_blend_attachments = vec![
            vk::PipelineColorBlendAttachmentState::default()
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
                .alpha_blend_op(vk::BlendOp::ADD),
        ];

        let mut create_info = GraphicsPipelineCreateInfo::default();
        create_info
            .shader_stages(ShaderStage::iter().map(|stage| stage.value().clone()).collect_vec())
            .vertex_attribute(ImGuiVertex::vertex_input_attributes())
            .vertex_binding(ImGuiVertex::vertex_input_bindings())
            .cull_mode(vk::CullModeFlags::NONE, vk::FrontFace::CLOCKWISE)
            .color_blend(color_blend_attachments, [0.0; 4])
            .depth_test(Some(vk::CompareOp::ALWAYS), false, false)
            // TODO 这里不应该由 depth
            .attach_info(vec![color_format], None, None);

        let pipeline =
            GraphicsPipeline::new(render_context.device_functions(), &create_info, pipeline_layout.clone(), "uipass");

        Self {
            pipeline,
            pipeline_layout,
            bindless_mgr,
        }
    }

    pub fn draw(
        &self,
        render_context: &RenderContext,
        canvas_color_view: vk::ImageView,
        canvas_extent: vk::Extent2D,
        cmd: &CommandBuffer,
        gui: &mut Gui,
        frame_label: FrameLabel,
    ) {
        // TODO mesh 应该放在 gui pass 中管理
        let color_attach_info = vk::RenderingAttachmentInfo::default()
            .image_view(canvas_color_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            })
            .store_op(vk::AttachmentStoreOp::STORE);

        let render_info = vk::RenderingInfo::default()
            .layer_count(1)
            .render_area(canvas_extent.into())
            .color_attachments(std::slice::from_ref(&color_attach_info));

        let mesh;
        let draw_data;
        let get_texture_key;
        if let Some(r) = gui.imgui_render(render_context, cmd, frame_label) {
            (mesh, draw_data, get_texture_key) = r;
        } else {
            log::warn!("No ImGui draw data available, skipping GUI pass.");
            return;
        }

        let viewport = vk::Viewport {
            width: draw_data.framebuffer_scale[0] * draw_data.display_size[0],
            height: draw_data.framebuffer_scale[1] * draw_data.display_size[1],
            min_depth: 0.0,
            ..Default::default()
        };

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
            &[self.bindless_mgr.borrow().bindless_descriptor_sets[*frame_label].handle()],
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

                        // 加载 texture，如果和上一个 command 使用的 texture
                        // 不是同一个，则需要重新加载
                        if Some(texture_id) != last_texture_id {
                            let texture_key = get_texture_key(texture_id);
                            let texture_handle = bindless_mgr
                                .get_texture_handle(&texture_key)
                                .unwrap_or_else(|| panic!("Texture not found: {}", texture_key));

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
