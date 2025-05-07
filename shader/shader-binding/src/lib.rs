mod shader_bindings;

use bytemuck::Zeroable;
pub use shader_bindings::*;

impl From<glam::Vec3> for float3 {
    fn from(value: glam::Vec3) -> Self {
        float3 {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<glam::Vec4> for float4 {
    fn from(value: glam::Vec4) -> Self {
        float4 {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

impl From<glam::Mat4> for float4x4 {
    fn from(value: glam::Mat4) -> Self {
        float4x4 {
            col0: float4::from(value.x_axis),
            col1: float4::from(value.y_axis),
            col2: float4::from(value.z_axis),
            col3: float4::from(value.w_axis),
        }
    }
}

unsafe impl Zeroable for PushConstants {}
unsafe impl bytemuck::Pod for PushConstants {}
