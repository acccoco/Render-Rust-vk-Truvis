fn main() {
    gen_rust_binding();
}

fn gen_rust_binding() {
    // 创建自定义回调实现
    #[derive(Debug)]
    struct PodTraitAdder;

    impl bindgen::callbacks::ParseCallbacks for PodTraitAdder {
        fn item_name(&self, _original_name: &str) -> Option<String> {
            None
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

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .header("rust_ffi.hpp")
        .derive_default(true)
        // 添加 bytemuck 的 Pod 和 Zeroable traits
        // .raw_line("use bytemuck::{Pod, Zeroable};")
        // 添加自定义回调
        .parse_callbacks(Box::new(PodTraitAdder))
        // 同时保留 cargo 回调
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = std::path::PathBuf::from("src").join("shader_bindings.rs");
    bindings.write_to_file(out_path).expect("Couldn't write bindings!");
}
