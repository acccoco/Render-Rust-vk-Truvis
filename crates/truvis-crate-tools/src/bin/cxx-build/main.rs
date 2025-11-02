use truvis_crate_tools::init_log::init_log;

/// 去掉 windows 路径前缀 `\\?\`
///
/// 经过 canionicalize 的路径会带上这个前缀
fn path_without_win_prefix(p: &std::path::Path) -> &str {
    // p.to_str().unwrap().strip_prefix(r"\\?\").unwrap()
    p.to_str().unwrap()
}

/// cmake generate
fn cmake_config(cmake_project: &std::path::Path) {
    let build_path = cmake_project.join("build");
    let vcpkg_root = std::env::var("VCPKG_ROOT").unwrap();

    let args = [
        "-DCMAKE_CONFIGURATION_TYPES=Debug;Release",
        &format!("-DCMAKE_TOOLCHAIN_FILE={}", vcpkg_root),
        "-Thost=x64",
        "-Ax64",
        "-G",
        "Visual Studio 17 2022",
        "-S",
        path_without_win_prefix(cmake_project),
        "-B",
        path_without_win_prefix(&build_path),
    ];

    log::info!("cmake config: {:#?}", args);
    std::process::Command::new("cmake").args(args).status().expect("Failed to run cmake");
}

enum BuildType {
    Debug,
    Release,
}
impl BuildType {
    fn cmake_output_dir(&self) -> &str {
        match self {
            BuildType::Debug => "Debug",
            BuildType::Release => "Release",
        }
    }
    fn cargo_output_dir(&self) -> &str {
        match self {
            BuildType::Debug => "debug",
            BuildType::Release => "release",
        }
    }
}

/// cmake 编译整个项目
fn cmake_build(cmake_project: &std::path::Path, build_type: BuildType) {
    let build_path = cmake_project.join("build");

    let args = [
        "--build",
        path_without_win_prefix(&build_path),
        "--config",
        build_type.cmake_output_dir(),
        "--parallel",
        "--target",
        "ALL_BUILD",
    ];

    log::info!("cmake build: {:#?}", args);
    std::process::Command::new("cmake").args(args).status().expect("Failed to run cmake build");
}

/// 将 cxx 编译的结果 copy 到 rust
fn copy_to_rust(cmake_project: &std::path::Path, cargo_target_dir: &std::path::Path, build_type: BuildType) {
    let cmake_output_path = cmake_project.join("build").join(build_type.cmake_output_dir());
    let cargo_output_path = cargo_target_dir.join(build_type.cargo_output_dir());

    let mut all_copy_files = Vec::new();
    for entry in std::fs::read_dir(cmake_output_path).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let source_path = entry.path();
        let suffix = source_path.extension().unwrap_or_default();

        // 需要复制的文件：.dll, .pdb, .lib
        if suffix != "dll" && suffix != "pdb" && suffix != "lib" {
            continue;
        }

        all_copy_files.push(file_name.to_str().unwrap().to_string());

        std::fs::copy(&source_path, cargo_output_path.join(&file_name)).unwrap();
        std::fs::copy(&source_path, cargo_output_path.join("examples").join(&file_name)).unwrap();
    }

    log::info!("Copied files to {}: {:#?}", cargo_output_path.display(), all_copy_files);
}

fn main() {
    init_log();

    // {workspace}/crates/{current_crate}
    let crate_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = crate_dir.parent().unwrap().parent().unwrap();
    log::info!("workspace_dir: {}", path_without_win_prefix(&crate_dir));
    log::info!("crate_dir: {}", path_without_win_prefix(workspace_dir));

    let cmake_project = workspace_dir.join("crates").join("truvis-cxx").join("cxx");

    cmake_config(&cmake_project);

    cmake_build(&cmake_project, BuildType::Debug);
    cmake_build(&cmake_project, BuildType::Release);

    copy_to_rust(&cmake_project, &workspace_dir.join("target"), BuildType::Debug);
    copy_to_rust(&cmake_project, &workspace_dir.join("target"), BuildType::Release);
}
