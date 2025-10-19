use ash::vk;
use truvis_rhi::resources::special_buffers::vertex_buffer::VertexLayout;

/// SoA 的顶点 buffer 布局，包含：Positions, Normals, Tangents, UVs
pub struct VertexLayoutSoA3D;
impl VertexLayout for VertexLayoutSoA3D {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![
            // positions
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<glam::Vec3>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            // normals
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: size_of::<glam::Vec3>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            // tangents
            vk::VertexInputBindingDescription {
                binding: 2,
                stride: size_of::<glam::Vec3>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            // uvs
            vk::VertexInputBindingDescription {
                binding: 3,
                stride: size_of::<glam::Vec2>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
        ]
    }

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            // positions
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            // normals
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            // tangents
            vk::VertexInputAttributeDescription {
                binding: 2,
                location: 2,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            // uvs
            vk::VertexInputAttributeDescription {
                binding: 3,
                location: 3,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
        ]
    }

    fn pos3d_attribute() -> (u32, u32) {
        (size_of::<glam::Vec3>() as u32, 0)
    }

    fn buffer_size(vertex_cnt: usize) -> usize {
        vertex_cnt * (size_of::<glam::Vec3>() * 3 + size_of::<glam::Vec2>())
    }
}

// 所有的顶点属性放在同一个 buffer 中，计算不同属性的偏移量
impl VertexLayoutSoA3D {
    pub fn get_vertex_buffer_offset(vertex_cnt: usize) -> [vk::DeviceSize; 4] {
        [
            Self::get_position_offset(vertex_cnt),
            Self::get_normal_offset(vertex_cnt),
            Self::get_tangent_offset(vertex_cnt),
            Self::get_uv_offset(vertex_cnt),
        ]
    }

    #[inline]
    pub fn get_position_offset(_vertex_cnt: usize) -> vk::DeviceSize {
        0
    }
    #[inline]
    pub fn get_normal_offset(vertex_cnt: usize) -> vk::DeviceSize {
        Self::get_position_offset(vertex_cnt) + (vertex_cnt * size_of::<glam::Vec3>()) as vk::DeviceSize
    }
    #[inline]
    pub fn get_tangent_offset(vertex_cnt: usize) -> vk::DeviceSize {
        Self::get_normal_offset(vertex_cnt) + (vertex_cnt * size_of::<glam::Vec3>()) as vk::DeviceSize
    }
    #[inline]
    pub fn get_uv_offset(vertex_cnt: usize) -> vk::DeviceSize {
        Self::get_tangent_offset(vertex_cnt) + (vertex_cnt * size_of::<glam::Vec3>()) as vk::DeviceSize
    }
}
