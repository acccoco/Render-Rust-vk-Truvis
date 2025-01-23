#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex
{
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
impl Vertex
{
    const fn new(pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self
    {
        Self { pos, normal, uv }
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
pub struct ShapeFloor;
impl ShapeFloor
{
    const VERTEX_A: Vertex = Vertex::new([5.0, 0.0, 5.0], [0.0, 1.0, 0.0], [1.0, 0.0]);
    const VERTEX_B: Vertex = Vertex::new([-5.0, 0.0, 5.0], [0.0, 1.0, 0.0], [0.0, 0.0]);
    const VERTEX_C: Vertex = Vertex::new([-5.0, 0.0, -5.0], [0.0, 1.0, 0.0], [0.0, 1.0]);
    const VERTEX_D: Vertex = Vertex::new([5.0, 0.0, -5.0], [0.0, 1.0, 0.0], [1.0, 1.0]);

    pub const VERTICES: [Vertex; 6] = [
        Self::VERTEX_A,
        Self::VERTEX_B,
        Self::VERTEX_C,
        Self::VERTEX_A,
        Self::VERTEX_C,
        Self::VERTEX_D,
    ];
}


/// Y-up, X-Right, Right hand 的坐标系中：
pub struct ShapeBox;
impl ShapeBox
{
    const TOP_A: Vertex = Vertex::new([0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [1.0, 0.0]);
    const TOP_B: Vertex = Vertex::new([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]);
    const TOP_C: Vertex = Vertex::new([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 1.0]);
    const TOP_D: Vertex = Vertex::new([0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [1.0, 1.0]);

    const BOTTOM_A: Vertex = Vertex::new([0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [1.0, 0.0]);
    const BOTTOM_B: Vertex = Vertex::new([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [0.0, 0.0]);
    const BOTTOM_C: Vertex = Vertex::new([-0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [0.0, 1.0]);
    const BOTTOM_D: Vertex = Vertex::new([0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [1.0, 1.0]);

    const NEAR_A: Vertex = Vertex::new([0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 0.0]);
    const NEAR_B: Vertex = Vertex::new([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 0.0]);
    const NEAR_C: Vertex = Vertex::new([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 1.0]);
    const NEAR_D: Vertex = Vertex::new([0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 1.0]);

    const FAR_A: Vertex = Vertex::new([0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 0.0]);
    const FAR_B: Vertex = Vertex::new([-0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0]);
    const FAR_C: Vertex = Vertex::new([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 1.0]);
    const FAR_D: Vertex = Vertex::new([0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 1.0]);

    const LEFT_A: Vertex = Vertex::new([-0.5, 0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 0.0]);
    const LEFT_B: Vertex = Vertex::new([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 0.0]);
    const LEFT_C: Vertex = Vertex::new([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 1.0]);
    const LEFT_D: Vertex = Vertex::new([-0.5, -0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 1.0]);

    const RIGHT_A: Vertex = Vertex::new([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 0.0]);
    const RIGHT_B: Vertex = Vertex::new([0.5, 0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 0.0]);
    const RIGHT_C: Vertex = Vertex::new([0.5, -0.5, -0.5], [1.0, 0.0, 0.0], [0.0, 1.0]);
    const RIGHT_D: Vertex = Vertex::new([0.5, -0.5, 0.5], [1.0, 0.0, 0.0], [1.0, 1.0]);

    pub const VERTICES: [Vertex; 36] = [
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
