use cmake::Config;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=cxx/CMakeLists.txt");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cxx/vcpkg.json");
    println!("cargo:rerun-if-changed=cxx/include");
    println!("cargo:rerun-if-changed=cxx/src");

    // 目录结构
    // $OUTDIR = $PROJECT/target/debug/build/$CRATE-$HASH/out
    // 其中：build/Debug 或者 build/Release 就是存放 lib, dll, exe, pdb 的位置

    // 编译 CMake 项目
    build_cmake_project();

    // 复制 DLL 文件
    copy_dll_files();

    // 将自动绑定文件写入到当前项目中
    gen_rust_binding();
}

/// 当前项目的 target 文件夹
fn _target_dir() -> PathBuf {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let debug_dir = out_dir.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap();
    debug_dir.to_path_buf()
}

/// cmake 项目编译结果的文件夹，放置 exe, lib, dll
fn cxx_bin_dir() -> PathBuf {
    std::path::PathBuf::from(format!(
        "{}/build/{}",
        std::env::var("OUT_DIR").unwrap(),
        if cfg!(debug_assertions) { "Debug" } else { "Release" }
    ))
}

/// rust 项目编译结果的文件夹，放置 exe, d, dll
fn rust_bin_dir() -> PathBuf {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let debug_dir = out_dir.parent().unwrap().parent().unwrap().parent().unwrap();
    debug_dir.to_path_buf()
}

/// 编译 CMake 项目
fn build_cmake_project() {
    // 配置 CMake 项目
    Config::new("cxx")
        .define(
            "CMAKE_TOOLCHAIN_FILE",
            format!("{}/scripts/buildsystems/vcpkg.cmake", std::env::var("VCPKG_ROOT").unwrap()),
        )
        .build();

    // 用于找到 dll 所需的引导 lib
    println!("cargo:rustc-link-search=native={}", cxx_bin_dir().display());
    println!("cargo:rustc-link-lib=static={}", "truvis-assimp");
}

/// 复制 DLL 文件到目标目录
fn copy_dll_files() {
    let rust_bin_dir = rust_bin_dir();

    // 使用 cargo:warning= 前缀让消息显示在构建输出中
    // println!("cargo:warning=Target directory: {}", target_dir);

    // 只需要复制 dll 文件到 target/debug 目录下
    for entry in std::fs::read_dir(cxx_bin_dir()).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let source_path = entry.path();
        if source_path.extension().unwrap_or_default() != "dll" {
            continue;
        }
        let destination_path = rust_bin_dir.join(file_name);
        std::fs::copy(source_path, destination_path).unwrap();
    }
}

/// 读取 c++ 头文件（只能有一个），输出到当前 crate 中
fn gen_rust_binding() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("cxx/include/lib.hpp")
        // .dynamic_library_name("TruvisAssimp")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = std::path::PathBuf::from("src").join("bindings.rs");
    bindings.write_to_file(out_path).expect("Couldn't write bindings!");
}
