use crate::outer_app::OuterApp;
use crate::outer_app::async_load_test::async_pass::AsyncPass;
use imgui::Ui;
use truvis_asset::handle::AssetTextureHandle;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::resources::special_buffers::index_buffer::GfxIndex32Buffer;
use truvis_gfx::resources::vertex_layout::soa_3d::VertexLayoutSoA3D;
use truvis_render_graph::render_context::RenderContext;
use truvis_renderer::platform::camera::Camera;
use truvis_renderer::renderer::Renderer;
use truvis_scene::components::geometry::RtGeometry;

#[derive(Default)]
pub struct AsyncLoadTest {
    pipeline: Option<AsyncPass>,
    quad: Option<RtGeometry>,
    texture_handle: Option<AssetTextureHandle>,
}

impl AsyncLoadTest {
    fn create_quad() -> RtGeometry {
        let positions = vec![
            glam::vec3(-0.5, -0.5, 0.0),
            glam::vec3(0.5, -0.5, 0.0),
            glam::vec3(0.5, 0.5, 0.0),
            glam::vec3(-0.5, 0.5, 0.0),
        ];
        let normals = vec![glam::Vec3::Z; 4];
        let tangents = vec![glam::Vec3::X; 4];
        let uvs = vec![
            glam::vec2(0.0, 0.0),
            glam::vec2(1.0, 0.0),
            glam::vec2(1.0, 1.0),
            glam::vec2(0.0, 1.0),
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];

        let vertex_buffer = VertexLayoutSoA3D::create_vertex_buffer(&positions, &normals, &tangents, &uvs, "quad-vb");
        let index_buffer = GfxIndex32Buffer::new_with_data(&indices, "quad-ib");

        RtGeometry {
            vertex_buffer,
            index_buffer,
        }
    }
}

impl OuterApp for AsyncLoadTest {
    fn init(&mut self, renderer: &mut Renderer, _camera: &mut Camera) {
        log::info!("Async Load Test init.");

        // Load a texture
        let texture_path = TruvisPath::resources_path_str("uv_checker.png");
        let texture_handle = renderer.asset_hub.load_texture(texture_path.into());

        self.pipeline = Some(AsyncPass::new(
            &renderer.render_context.global_descriptor_sets,
            &renderer.render_context.frame_settings,
            &mut renderer.cmd_allocator,
        ));
        self.quad = Some(Self::create_quad());
        self.texture_handle = Some(texture_handle);
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self, _frame_context2: &RenderContext) {
        // let texture_id = FrameContext::bindless_manager()
        //     .get_texture_handle2(self.texture_handle)
        //     .map(|h| h.index as u32)
        //     .unwrap_or(0);
        //
        // self.pipeline.render(&self.quad, texture_id);
    }
}
