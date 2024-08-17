#[repr(C)]
pub struct StaticVertex
{
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv: [f32; 2],
}


#[derive(Default)]
pub struct StaticMeshData
{
    pub positions: Vec<[f32; 3]>,
    pub normal: Vec<[f32; 3]>,
    pub tangent: Vec<[f32; 4]>,
    pub uv: Vec<[f32; 2]>,
    pub index: Vec<u32>,
}
