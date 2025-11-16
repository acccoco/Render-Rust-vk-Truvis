use crate::components::geometry::GeometrySoA3D;
use crate::vertex::soa_3d::VertexLayoutSoA3D;
use truvis_gfx::resources::special_buffers::index_buffer::Index32Buffer;

/// 坐标系：RightHand, X-Right, Y-Up
///
/// 位于 XY 平面上的矩形，法线 +Z
///
/// 三角形绕序: CCW: ABC, ACD
///
/// ```text
///          y^
///           |
///      D----+----C
///       |   |   |
///       |   |   |
/// ------|---+---|------>x
///       |   |   |
///       |   |   |
///      A----+----B
///           |
/// ```
pub struct RectSoA {}
impl RectSoA {
    // 4 个顶点：从 aos_pos_color 的 RECTANGLE_VERTEX_DATA 提取位置
    const POSITIONS: [glam::Vec3; 4] = [
        glam::vec3(-1.0, 1.0, 0.0),  // A (左下)
        glam::vec3(1.0, 1.0, 0.0),   // B (右下)
        glam::vec3(1.0, -1.0, 0.0),  // C (右上)
        glam::vec3(-1.0, -1.0, 0.0), // D (左上)
    ];

    // 法线都指向 Z+ (朝向观察者)
    const NORMALS: [glam::Vec3; 4] = [
        glam::vec3(0.0, 0.0, 1.0),
        glam::vec3(0.0, 0.0, 1.0),
        glam::vec3(0.0, 0.0, 1.0),
        glam::vec3(0.0, 0.0, 1.0),
    ];

    // UV 坐标：标准纹理映射
    const UVS: [glam::Vec2; 4] = [
        glam::vec2(0.0, 1.0), // A (左下)
        glam::vec2(1.0, 1.0), // B (右下)
        glam::vec2(1.0, 0.0), // C (右上)
        glam::vec2(0.0, 0.0), // D (左上)
    ];

    // 切线指向 X+ (U 轴方向)
    const TANGENTS: [glam::Vec3; 4] = [
        glam::vec3(1.0, 0.0, 0.0),
        glam::vec3(1.0, 0.0, 0.0),
        glam::vec3(1.0, 0.0, 0.0),
        glam::vec3(1.0, 0.0, 0.0),
    ];

    // 两个三角形：ABC, ACD
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
            "rect-vertex-buffer",
        );

        let index_buffer = Index32Buffer::new(Self::INDICES.len(), "rect-index-buffer");
        index_buffer.transfer_data_sync(&Self::INDICES);

        GeometrySoA3D {
            vertex_buffer,
            index_buffer,
        }
    }
}
