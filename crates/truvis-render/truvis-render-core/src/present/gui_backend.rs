//! 参考 imgui-rs-vulkan-renderer

use crate::present::gui_mesh::GuiMesh;
use imgui::{DrawData, FontAtlasTexture, TextureId};
use std::collections::HashMap;
use truvis_gfx::commands::command_buffer::GfxCommandBuffer;
use truvis_gfx::{basic::color::LabelColor, gfx::Gfx, resources::image::GfxImage};
use truvis_render_base::bindless_manager::BindlessManager;
use truvis_render_base::cmd_allocator::CmdAllocator;
use truvis_render_base::frame_counter::FrameCounter;
use truvis_render_base::pipeline_settings::FrameLabel;
use truvis_resource::gfx_resource_manager::GfxResourceManager;
use truvis_resource::handles::GfxTextureHandle;
use truvis_resource::texture::GfxTexture;

// TODO 这个东西和 GuiHost 的重复了
const FONT_TEXTURE_ID: usize = 0;
const RENDER_IMAGE_ID: usize = 1;

pub struct GuiBackend {
    pub cmds: [GfxCommandBuffer; FrameCounter::fif_count()],

    /// 存放多帧 imgui 的 mesh 数据
    pub gui_meshes: [GuiMesh; FrameCounter::fif_count()],

    render_texture_handle: Option<GfxTextureHandle>,
    font_texture_handle: Option<GfxTextureHandle>,
    font_tex_id: TextureId,

    pub tex_map: HashMap<TextureId, GfxTextureHandle>,
}
// 创建过程
impl GuiBackend {
    pub fn new(cmd_allocator: &mut CmdAllocator) -> Self {
        let gui_meshes = FrameCounter::frame_labes().map(GuiMesh::new);

        let cmds = FrameCounter::frame_labes()
            .map(|frame_label| cmd_allocator.alloc_command_buffer(frame_label, "window-present"));

        Self {
            gui_meshes,
            render_texture_handle: None,
            font_texture_handle: None,
            font_tex_id: TextureId::new(0),
            cmds,

            tex_map: Default::default(),
        }
    }

    pub fn register_font(
        &mut self,
        bindless_manager: &mut BindlessManager,
        gfx_resource_manager: &mut GfxResourceManager,
        font_atlas: FontAtlasTexture,
        font_tex_id: TextureId,
    ) {
        let image = GfxImage::from_rgba8(font_atlas.width, font_atlas.height, font_atlas.data, "imgui-fonts");
        let fonts_texture = GfxTexture::new(image, "imgui-fonts");

        let fonts_texture_handle = gfx_resource_manager.register_texture(fonts_texture);
        bindless_manager.register_srv_with_texture(fonts_texture_handle);

        self.font_texture_handle = Some(fonts_texture_handle);
        self.font_tex_id = font_tex_id;
    }
}
// tools
impl GuiBackend {
    pub fn register_render_texture(&mut self, texture_handle: GfxTextureHandle) {
        self.render_texture_handle = Some(texture_handle);
    }

    // TODO 这个函数设计的非常别扭
    /// # Phase: Render
    ///
    /// 使用 imgui 将 ui 操作编译为 draw data；构建 draw 需要的 mesh 数据
    pub fn prepare_render_data(&mut self, draw_data: &DrawData, frame_label: FrameLabel) {
        Gfx::get().gfx_queue().begin_label("[ui-pass]create-mesh", LabelColor::COLOR_STAGE);
        self.gui_meshes[*frame_label].grow_if_needed(draw_data);
        self.gui_meshes[*frame_label].fill_vertex_buffer(draw_data);
        self.gui_meshes[*frame_label].fill_index_buffer(draw_data);
        Gfx::get().gfx_queue().end_label();

        self.tex_map = HashMap::from([
            (imgui::TextureId::new(RENDER_IMAGE_ID), self.render_texture_handle.unwrap()),
            (imgui::TextureId::new(FONT_TEXTURE_ID) as imgui::TextureId, self.font_texture_handle.unwrap()),
        ]);
    }
}
