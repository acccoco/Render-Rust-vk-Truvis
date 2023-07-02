pub struct Material
{
    pub material_params: Vec<MaterialParam>,
}


pub enum MaterialValue
{
    Int(i32),
    Vector(glam::Vec4),
    Float(f32),
    Str(String),
}

pub struct MaterialParam
{
    pub name: String,
    pub value: MaterialValue,
}
