use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_asset::handle::AssetTextureHandle;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::resources::special_buffers::index_buffer::GfxIndex32Buffer;
use truvis_model_manager::components::geometry::GeometrySoA3D;
use truvis_model_manager::vertex::soa_3d::VertexLayoutSoA3D;
use truvis_render::core::frame_context::FrameContext;
use truvis_render::core::renderer::Renderer;
use truvis_render::platform::camera::Camera;

mod async_pass;
use async_pass::AsyncPass;

struct AsyncLoadTest {
    pipeline: AsyncPass,
    quad: GeometrySoA3D,
    texture_handle: AssetTextureHandle,
}

impl AsyncLoadTest {
    fn create_quad() -> GeometrySoA3D {
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

        GeometrySoA3D {
            vertex_buffer,
            index_buffer,
        }
    }
}

impl OuterApp for AsyncLoadTest {
    fn init(_renderer: &mut Renderer, _camera: &mut Camera) -> Self {
        log::info!("Async Load Test init.");

        // Load a texture
        let texture_path = TruvisPath::resources_path("uv_checker.png");
        let texture_handle = FrameContext::asset_hub_mut().load_texture(texture_path.into());

        Self {
            pipeline: AsyncPass::new(&FrameContext::get().frame_settings()),
            quad: Self::create_quad(),
            texture_handle,
        }
    }

    fn draw_ui(&mut self, _ui: &Ui) {}

    fn draw(&self) {
        let texture_id = FrameContext::bindless_manager()
            .get_texture_handle2(self.texture_handle)
            .map(|h| h.index as u32)
            .unwrap_or(0);

        self.pipeline.render(&self.quad, texture_id);
    }
}

fn main() {
    TruvisApp::<AsyncLoadTest>::run();
}
