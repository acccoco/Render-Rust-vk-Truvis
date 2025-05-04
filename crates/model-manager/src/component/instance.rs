pub struct SimpleInstance {
    pub transform: glam::Mat4,

    pub meshes: Vec<uuid::Uuid>,
    pub mats: Vec<uuid::Uuid>,
}
