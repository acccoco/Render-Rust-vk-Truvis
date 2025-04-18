use itertools::all;

#[repr(C)]
pub struct StaticVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}

#[derive(Default)]
pub struct MeshBuilder {
    pub positions: Vec<f32>, // (x, y ,z)
    pub normal: Vec<f32>,    // (x, y, z)
    pub tangent: Vec<f32>,   // (x, y, z, w)
    pub uv: Vec<f32>,        // (u, v)

    /// 第一层表示 submesh
    pub index: Vec<Vec<u32>>,
}

impl MeshBuilder {
    /// 判断 mesh 是否符合以下调节
    /// * 有 index，且每个 submesh 都是三角面
    /// * 有 vertex，且保证 normal，tangent，uv 的数量要么是 0，要么和 vertex 数量一致
    pub fn is_valid(&self) -> bool {
        let vertex_cnt = self.positions.len() / 3;
        !self.index.is_empty()
            && all(self.index.iter(), |submesh| !submesh.is_empty() && submesh.len() % 3 == 0)
            && vertex_cnt != 0
            && self.positions.len() % 3 == 0
            && (self.normal.is_empty() || self.normal.len() == vertex_cnt * 3)
            && (self.tangent.is_empty() || self.tangent.len() == vertex_cnt * 4)
            && (self.uv.is_empty() || self.uv.len() == vertex_cnt * 2)
    }
}
