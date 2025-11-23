use crate::components::geometry::GeometrySoA3D;
use crate::vertex::soa_3d::VertexLayoutSoA3D;
use truvis_gfx::resources::special_buffers::index_buffer::GfxIndex32Buffer;

/// 坐标系：RightHandle, X-Right, Y-Up
///
/// 面片位于 XZ 平面上，朝向 +Y
///
/// 面片长宽都是 1
///
/// 三角形绕序 CCW: ABC, ACD
///
/// 左上角 B 视为 UV 起点
///
/// ```text
///            z^
///             |
///      B-------------A
///       |     |     |
/// ------|-----|-----|------>x
///       |     |     |
///      C-------------D
///             |
/// ```
pub struct FloorSoA {}
impl FloorSoA {
    const POSITIONS: [glam::Vec3; 4] = [
        glam::vec3(1.0, 0.0, 1.0),   // A
        glam::vec3(1.0, 0.0, -1.0),  // B
        glam::vec3(-1.0, 0.0, -1.0), // C
        glam::vec3(-1.0, 0.0, 1.0),  // D
    ];
    const NORMALS: [glam::Vec3; 4] = [glam::vec3(0.0, 1.0, 0.0); _];
    const UVS: [glam::Vec2; 4] = [
        glam::vec2(1.0, 0.0), // A
        glam::vec2(0.0, 0.0), // B
        glam::vec2(0.0, 1.0), // C
        glam::vec2(1.0, 1.0), // D
    ];
    const TANGENTS: [glam::Vec3; 4] = [glam::vec3(1.0, 0.0, 0.0); _];
    const INDICES: [u32; 6] = [
        0, 1, 2, // ABC
        0, 2, 3, // ACD
    ];

    pub fn create_mesh() -> GeometrySoA3D {
        let vertex_buffer = VertexLayoutSoA3D::create_vertex_buffer(
            &Self::POSITIONS,
            &Self::NORMALS,
            &Self::TANGENTS,
            &Self::UVS,
            "floor-vertex-buffer",
        );

        let index_buffer = GfxIndex32Buffer::new(Self::INDICES.len(), "floor-index-buffer");
        index_buffer.transfer_data_sync(&Self::INDICES);

        GeometrySoA3D {
            vertex_buffer,
            index_buffer,
        }
    }
}
