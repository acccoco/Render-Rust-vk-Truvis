// build.rs
use cmake::Config;
use std::env;
use std::path::PathBuf;

fn main() {
    // 获取输出目录
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 设置 vcpkg 的路径
    let vcpkg_root = env::var("VCPKG_ROOT").unwrap_or_else(|_| "vcpkg".to_string());
    // let vcpkg_triplet = env::var("VCPKG_DEFAULT_TRIPLET").unwrap_or_else(|_| "x64-linux".to_string()); // 根据你的平台调整

    // 配置 CMake 项目
    let dst = Config::new("cxx")
        .define("CMAKE_TOOLCHAIN_FILE", format!("{}/scripts/buildsystems/vcpkg.cmake", vcpkg_root))
        // .define("VCPKG_TARGET_TRIPLET", &vcpkg_triplet)
        // .define("CMAKE_BUILD_TYPE", "Release")  // 可以根据需要设置其他 CMake 定义
        .build();

    // 将生成的库链接到 Rust 项目
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=my_third_party_lib"); // 替换为你的库名
}
