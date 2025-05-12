mod _shader_bindings;

pub mod shader {
    pub use crate::_shader_bindings::*;
}

mod slang_traits {
    use crate::_shader_bindings::*;

    impl From<glam::UVec2> for uint2 {
        fn from(value: glam::UVec2) -> Self {
            uint2 { x: value.x, y: value.y }
        }
    }

    impl From<glam::UVec3> for uint3 {
        fn from(value: glam::UVec3) -> Self {
            uint3 {
                x: value.x,
                y: value.y,
                z: value.z,
            }
        }
    }

    impl From<glam::UVec4> for uint4 {
        fn from(value: glam::UVec4) -> Self {
            uint4 {
                x: value.x,
                y: value.y,
                z: value.z,
                w: value.w,
            }
        }
    }

    impl From<glam::IVec2> for int2 {
        fn from(value: glam::IVec2) -> Self {
            int2 { x: value.x, y: value.y }
        }
    }

    impl From<glam::IVec3> for int3 {
        fn from(value: glam::IVec3) -> Self {
            int3 {
                x: value.x,
                y: value.y,
                z: value.z,
            }
        }
    }

    impl From<glam::IVec4> for int4 {
        fn from(value: glam::IVec4) -> Self {
            int4 {
                x: value.x,
                y: value.y,
                z: value.z,
                w: value.w,
            }
        }
    }

    impl From<glam::Vec2> for float2 {
        fn from(value: glam::Vec2) -> Self {
            float2 { x: value.x, y: value.y }
        }
    }

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
}
