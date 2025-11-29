use ash::vk;
use imgui::Ui;
use truvis_app::app::TruvisApp;
use truvis_app::outer_app::OuterApp;
use truvis_asset::handle::TextureHandle;
use truvis_crate_tools::resource::TruvisPath;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::handles::IndexBufferHandle;
use truvis_gfx::resources::resource_data::BufferType;
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
    texture_handle: TextureHandle,
}

impl AsyncLoadTest {
    fn create_index_buffer(indices: &[u32], name: &str) -> IndexBufferHandle {
        let mut rm = Gfx::get().resource_manager();
        let index_count = indices.len();
        let index_buffer = rm.create_index_buffer::<u32>(index_count, name);

        let buffer_size = std::mem::size_of_val(indices) as u64;

        let stage_buffer_handle = rm.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            format!("{}-stage", name),
        );

        {
            let stage_buffer = rm.get_buffer_mut(stage_buffer_handle).unwrap();
            if let Some(ptr) = stage_buffer.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(indices.as_ptr(), ptr as *mut u32, index_count);
                }
            }
        }

        let src_buffer = rm.get_buffer(stage_buffer_handle).unwrap().buffer;
        let dst_buffer = rm.get_index_buffer(index_buffer).unwrap().buffer;

        Gfx::get().one_time_exec(
            |cmd| {
                let copy_region = vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size: buffer_size,
                };
                unsafe {
                    Gfx::get().gfx_device().cmd_copy_buffer(cmd.vk_handle(), src_buffer, dst_buffer, &[copy_region]);
                }
            },
            "upload_index_buffer",
        );

        rm.destroy_buffer_immediate(stage_buffer_handle);

        index_buffer
    }

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
        let index_buffer = Self::create_index_buffer(&indices, "quad-ib");

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
            .get_asset_texture_handle(self.texture_handle)
            .map(|h| h.index as u32)
            .unwrap_or(0);

        self.pipeline.render(&self.quad, texture_id);
    }
}

fn main() {
    TruvisApp::<AsyncLoadTest>::run();
}
