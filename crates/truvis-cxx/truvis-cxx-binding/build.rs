/// 读取 c++ 头文件（只能有一个），输出到当前 crate 中
fn gen_rust_binding() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .header("../../../cxx/truvixx-interface/include/TruvixxInterface/lib.hpp")
        .clang_args([
            "-I../../../cxx/truvixx-assimp/include",
            "-I../../../cxx/truvixx-interface/include",
        ])
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .raw_line("#![allow(clippy::all)]")
        .raw_line("#![allow(warnings)]")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .enable_cxx_namespaces()
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = std::path::PathBuf::from("src").join("_ffi_bindings.rs");
    bindings.write_to_file(out_path).expect("Couldn't write bindings!");
}

/// 强制执行的方法: touch build.rs; cargo build
fn main() {
    println!("cargo:rerun-if-changed=cxx/CMakeLists.txt");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cxx/vcpkg.json");
    println!("cargo:rerun-if-changed=cxx/truvixx-assimp");
    println!("cargo:rerun-if-changed=cxx/truvixx-interface");

    // 将自动绑定文件写入到当前项目中
    gen_rust_binding();

    // 指定需要静态链接的符号文件 .lib
    // {workspace}/crates/{current_crate}
    let crate_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let build_type = std::env::var("PROFILE").unwrap();

    // 手动找到 workspace 的路径，依赖当前 crate 的相对路径
    let workspace_dir = crate_dir.parent().unwrap().parent().unwrap().parent().unwrap();
    let cargo_build_dir = workspace_dir.join("target").join(build_type);
    println!("cargo:rustc-link-search=native={}", cargo_build_dir.display());
    let libs = ["truvixx-interface"];
    for lib in libs {
        println!("cargo:rustc-link-lib=static={}", lib);
    }
}
