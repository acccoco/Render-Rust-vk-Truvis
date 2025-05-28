fn main() {
    gen_rust_binding();
}

// 创建自定义回调实现
#[derive(Debug)]
struct ModifyAdder;

impl bindgen::callbacks::ParseCallbacks for ModifyAdder {
    fn item_name(&self, _original_name: &str) -> Option<String> {
        match _original_name {
            "uint" => Some("Uint".to_string()),
            "uint2" => Some("Uint2".to_string()),
            "uint3" => Some("Uint3".to_string()),
            "uint4" => Some("Uint4".to_string()),

            "int2" => Some("Int2".to_string()),
            "int3" => Some("Int3".to_string()),
            "int4" => Some("Int4".to_string()),

            "float2" => Some("Float2".to_string()),
            "float3" => Some("Float3".to_string()),
            "float4" => Some("Float4".to_string()),

            "float4x4" => Some("Float4x4".to_string()),

            &_ => None,
        }
    }

    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo) -> Vec<String> {
        // 为结构体添加 Pod 和相关 traits
        if info.kind == bindgen::callbacks::TypeKind::Struct {
            vec![
                // "Clone".into(), //
                // "Copy".into(),  //
                "bytemuck::Pod".into(),      //
                "bytemuck::Zeroable".into(), //
            ]
        } else {
            vec![]
        }
    }
}

fn gen_rust_binding() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .header("rust_ffi.hpp")
        .derive_default(false)
        // 禁用 clippy 的检查
        .raw_line("#![allow(clippy::all)]")
        .enable_cxx_namespaces()
        // 添加自定义回调
        .parse_callbacks(Box::new(ModifyAdder))
        // 同时保留 cargo 回调
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = std::path::PathBuf::from("src").join("_shader_bindings.rs");
    bindings.write_to_file(out_path).expect("Couldn't write bindings!");
}
