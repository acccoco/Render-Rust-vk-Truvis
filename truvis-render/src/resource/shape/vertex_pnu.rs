use ash::vk;
use std::mem::offset_of;

/// AoS: Array of structures
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VertexPNUAoS {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
impl VertexPNUAoS {
    const fn new(pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self { pos, normal, uv }
    }

    pub fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn vertex_input_attriutes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(VertexPNUAoS, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(VertexPNUAoS, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(VertexPNUAoS, uv) as u32 * 2,
            },
        ]
    }

    pub fn shape_floor() -> &'static [VertexPNUAoS] {
        &ShapeFloor::VERTICES
    }

    pub fn shape_box() -> &'static [VertexPNUAoS] {
        &ShapeBox::VERTICES
    }
}

/// Y-up, Right hand 的坐标系中：
///
/// 面片位于 xz 平面上，朝向 y+（顶点顺序和法线保持一致）
///
/// 两个三角形的顺序为：ABC, ACD
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
struct ShapeFloor;
impl ShapeFloor {
    const VERTEX_A: VertexPNUAoS = VertexPNUAoS::new([5.0, 0.0, 5.0], [0.0, 1.0, 0.0], [1.0, 0.0]);
    const VERTEX_B: VertexPNUAoS = VertexPNUAoS::new([-5.0, 0.0, 5.0], [0.0, 1.0, 0.0], [0.0, 0.0]);
    const VERTEX_C: VertexPNUAoS = VertexPNUAoS::new([-5.0, 0.0, -5.0], [0.0, 1.0, 0.0], [0.0, 1.0]);
    const VERTEX_D: VertexPNUAoS = VertexPNUAoS::new([5.0, 0.0, -5.0], [0.0, 1.0, 0.0], [1.0, 1.0]);

    const VERTICES: [VertexPNUAoS; 6] = [
        Self::VERTEX_A,
        Self::VERTEX_B,
        Self::VERTEX_C,
        Self::VERTEX_A,
        Self::VERTEX_C,
        Self::VERTEX_D,
    ];
}

/// Y-up, X-Right, Right hand 的坐标系中：
struct ShapeBox;
impl ShapeBox {
    const TOP_A: VertexPNUAoS = VertexPNUAoS::new([0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [1.0, 0.0]);
    const TOP_B: VertexPNUAoS = VertexPNUAoS::new([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]);
    const TOP_C: VertexPNUAoS = VertexPNUAoS::new([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 1.0]);
    const TOP_D: VertexPNUAoS = VertexPNUAoS::new([0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [1.0, 1.0]);

    const BOTTOM_A: VertexPNUAoS = VertexPNUAoS::new([0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [1.0, 0.0]);
    const BOTTOM_B: VertexPNUAoS = VertexPNUAoS::new([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [0.0, 0.0]);
    const BOTTOM_C: VertexPNUAoS = VertexPNUAoS::new([-0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [0.0, 1.0]);
    const BOTTOM_D: VertexPNUAoS = VertexPNUAoS::new([0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [1.0, 1.0]);

    const NEAR_A: VertexPNUAoS = VertexPNUAoS::new([0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 0.0]);
    const NEAR_B: VertexPNUAoS = VertexPNUAoS::new([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 0.0]);
    const NEAR_C: VertexPNUAoS = VertexPNUAoS::new([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 1.0]);
    const NEAR_D: VertexPNUAoS = VertexPNUAoS::new([0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 1.0]);

    const FAR_A: VertexPNUAoS = VertexPNUAoS::new([0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 0.0]);
    const FAR_B: VertexPNUAoS = VertexPNUAoS::new([-0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0]);
    const FAR_C: VertexPNUAoS = VertexPNUAoS::new([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 1.0]);
    const FAR_D: VertexPNUAoS = VertexPNUAoS::new([0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 1.0]);

    const LEFT_A: VertexPNUAoS = VertexPNUAoS::new([-0.5, 0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 0.0]);
    const LEFT_B: VertexPNUAoS = VertexPNUAoS::new([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 0.0]);
    const LEFT_C: VertexPNUAoS = VertexPNUAoS::new([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 1.0]);
    const LEFT_D: VertexPNUAoS = VertexPNUAoS::new([-0.5, -0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 1.0]);

    const RIGHT_A: VertexPNUAoS = VertexPNUAoS::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 0.0]);
    const RIGHT_B: VertexPNUAoS = VertexPNUAoS::new([0.5, 0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 0.0]);
    const RIGHT_C: VertexPNUAoS = VertexPNUAoS::new([0.5, -0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 1.0]);
    const RIGHT_D: VertexPNUAoS = VertexPNUAoS::new([0.5, -0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 1.0]);

    const VERTICES: [VertexPNUAoS; 36] = [
        Self::TOP_A,
        Self::TOP_B,
        Self::TOP_C,
        Self::TOP_A,
        Self::TOP_C,
        Self::TOP_D,
        //
        Self::BOTTOM_A,
        Self::BOTTOM_C,
        Self::BOTTOM_B,
        Self::BOTTOM_A,
        Self::BOTTOM_D,
        Self::BOTTOM_C,
        //
        Self::NEAR_A,
        Self::NEAR_B,
        Self::NEAR_C,
        Self::NEAR_A,
        Self::NEAR_C,
        Self::NEAR_D,
        //
        Self::FAR_A,
        Self::FAR_C,
        Self::FAR_B,
        Self::FAR_A,
        Self::FAR_D,
        Self::FAR_C,
        //
        Self::LEFT_A,
        Self::LEFT_B,
        Self::LEFT_C,
        Self::LEFT_A,
        Self::LEFT_C,
        Self::LEFT_D,
        //
        Self::RIGHT_A,
        Self::RIGHT_C,
        Self::RIGHT_B,
        Self::RIGHT_A,
        Self::RIGHT_D,
        Self::RIGHT_C,
    ];
}
