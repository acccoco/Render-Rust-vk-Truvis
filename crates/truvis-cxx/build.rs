/// 强制执行的方法: touch build.rs; cargo build
fn main() {
    println!("cargo:rerun-if-changed=cxx/CMakeLists.txt");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cxx/vcpkg.json");
    println!("cargo:rerun-if-changed=cxx/include");
    println!("cargo:rerun-if-changed=cxx/src");

    // 编译 CMake 项目
    build_cmake_project();

    // 复制 DLL 文件
    copy_dll_files();

    // 将自动绑定文件写入到当前项目中
    gen_rust_binding();
}

/// # 默认目录结构
/// * cargo 默认的环境变量：${OUT_DIR} = $PROJECT/target/debug/build/$CRATE-$HASH/out
/// * 默认情况下，cmake 的 build 目录 = ${OUT_DIR}/build
/// * 其中：${OUT_DIR}/build/Debug 或者 ${OUT_DIR}/build/Release 就是存放 lib, dll, exe, pdb 的位置
struct CrateDirs;
impl CrateDirs {
    /// rust 整个 workspace 链接结果的文件夹，放置 exe, d, dll
    /// target/debug 或者 target/release
    fn rust_bin_dir() -> std::path::PathBuf {
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let debug_dir = out_dir.parent().unwrap().parent().unwrap().parent().unwrap();
        debug_dir.to_path_buf()
    }

    /// 当前 workspace 的 target 文件夹
    fn _rust_target_dir() -> std::path::PathBuf {
        Self::rust_bin_dir().parent().unwrap().to_path_buf()
    }

    /// 当前 crate 的文件夹，也就是 build.rs 所在的文件夹
    fn crate_dir() -> std::path::PathBuf {
        std::env::current_dir().unwrap()
    }
}

struct CMakeDirs;
impl CMakeDirs {
    /// 自定义的 cmake 输出目录，里面存放 cmake 的 build 文件夹
    fn cmake_custom_output_dir() -> std::path::PathBuf {
        CrateDirs::crate_dir().join("cargo-cmake-output")
    }

    /// cmake build 的文件夹，里面存放着 cmake 的构建结果
    fn cmake_build_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/build", Self::cmake_custom_output_dir().display()))
    }

    /// cmake build 文件夹下的 Debug 或 Release 目录，里面存放在 lib 以及 dll, pdb
    fn cmake_custom_output_lib_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(format!(
            "{}/{}",
            Self::cmake_build_dir().display(),
            if cfg!(debug_assertions) { "Debug" } else { "Release" }
        ))
    }
}

/// 编译 CMake 项目
fn build_cmake_project() {
    // 配置 CMake 项目
    cmake::Config::new("cxx")
        .define(
            "CMAKE_TOOLCHAIN_FILE",
            format!("{}/scripts/buildsystems/vcpkg.cmake", std::env::var("VCPKG_ROOT").unwrap()),
        )
        .out_dir(CMakeDirs::cmake_custom_output_dir())
        .build_target("ALL_BUILD")
        .build();

    println!("cargo:rustc-link-search=native={}", CMakeDirs::cmake_custom_output_lib_dir().display());

    let cxx_target = "truvis-assimp-cxx";
    println!("cargo:rustc-link-lib=static={}", cxx_target);
}

/// 复制 DLL 文件到目标目录
fn copy_dll_files() {
    // 使用 cargo:warning= 前缀让消息显示在构建输出中
    // println!("cargo:warning=src dir: {}", Dirs::cmake_custom_install_bin_dir().display());

    // 只需要复制 dll 文件到 target/debug 目录下
    for entry in std::fs::read_dir(CMakeDirs::cmake_custom_output_lib_dir()).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let source_path = entry.path();
        let suffix = source_path.extension().unwrap_or_default();
        if suffix != "dll" && suffix != "pdb" {
            continue;
        }

        std::fs::copy(&source_path, CrateDirs::rust_bin_dir().join(&file_name)).unwrap();
        std::fs::copy(&source_path, CrateDirs::rust_bin_dir().join("examples").join(&file_name)).unwrap();
    }
}

/// 读取 c++ 头文件（只能有一个），输出到当前 crate 中
fn gen_rust_binding() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .header("cxx/include/lib.hpp")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = std::path::PathBuf::from("src").join("_ffi_bindings.rs");
    bindings.write_to_file(out_path).expect("Couldn't write bindings!");
}
