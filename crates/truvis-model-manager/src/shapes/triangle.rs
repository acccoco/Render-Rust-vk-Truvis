use crate::components::geometry::GeometrySoA3D;
use crate::vertex::soa_3d::VertexLayoutSoA3D;
use ash::vk;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::resource_data::BufferType;

/// 坐标系：RightHand, X-Right, Y-Up
///
/// 位于 XY 平面上的正立三角形，法线 +Z
///
/// 三角形绕序: CCW
///
/// ```text
///          y^
///           |
///           C (red)
///          /|\
///         / | \
///        /  |  \
///       /   |   \
///      /    |    \
///     /     |     \
///    A------+------B---->x
/// (green)   |   (blue)
///           |
/// ```
pub struct TriangleSoA {}

impl TriangleSoA {
    // 3 个顶点：从 aos_pos_color 的 TRIANGLE_VERTEX_DATA 提取位置
    const POSITIONS: [glam::Vec3; 3] = [
        glam::vec3(-1.0, -1.0, 0.0), // A (左下, 绿色)
        glam::vec3(1.0, -1.0, 0.0),  // B (右下, 蓝色)
        glam::vec3(0.0, 1.0, 0.0),   // C (顶部, 红色)
    ];

    // 法线都指向 Z+ (朝向观察者)
    const NORMALS: [glam::Vec3; 3] = [
        glam::vec3(0.0, 0.0, 1.0),
        glam::vec3(0.0, 0.0, 1.0),
        glam::vec3(0.0, 0.0, 1.0),
    ];

    // UV 坐标：A(左下) B(右下) C(顶部)
    const UVS: [glam::Vec2; 3] = [
        glam::vec2(0.0, 1.0), // A
        glam::vec2(1.0, 1.0), // B
        glam::vec2(0.5, 0.0), // C
    ];

    // 切线指向 X+ (U 轴方向)
    const TANGENTS: [glam::Vec3; 3] = [
        glam::vec3(1.0, 0.0, 0.0),
        glam::vec3(1.0, 0.0, 0.0),
        glam::vec3(1.0, 0.0, 0.0),
    ];

    const INDICES: [u32; 3] = [0, 1, 2];

    pub fn create_mesh() -> GeometrySoA3D {
        let vertex_buffer = VertexLayoutSoA3D::create_vertex_buffer(
            &Self::POSITIONS,
            &Self::NORMALS,
            &Self::TANGENTS,
            &Self::UVS,
            "triangle-vertex-buffer",
        );

        let mut rm = Gfx::get().resource_manager();
        let index_buffer = rm.create_index_buffer::<u32>(Self::INDICES.len(), "triangle-index-buffer");

        // Upload data
        let stage_buffer_handle = rm.create_buffer(
            std::mem::size_of_val(&Self::INDICES) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            BufferType::Stage,
            "triangle-index-buffer-stage",
        );

        {
            let stage_buffer = rm.get_buffer_mut(stage_buffer_handle).unwrap();
            if let Some(ptr) = stage_buffer.mapped_ptr {
                unsafe {
                    std::ptr::copy_nonoverlapping(Self::INDICES.as_ptr(), ptr as *mut u32, Self::INDICES.len());
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
                    size: std::mem::size_of_val(&Self::INDICES) as u64,
                };
                unsafe {
                    Gfx::get().gfx_device().cmd_copy_buffer(cmd.vk_handle(), src_buffer, dst_buffer, &[copy_region]);
                }
            },
            "upload_index_buffer",
        );

        rm.destroy_buffer_immediate(stage_buffer_handle);

        GeometrySoA3D {
            vertex_buffer,
            index_buffer,
        }
    }
}
