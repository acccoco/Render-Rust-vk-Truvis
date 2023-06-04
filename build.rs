//! 注：如果 build 出现问题，可能是文件路径导致的。
//!
//! 可以单独建立一个 binary 项目，将 build.rs 内容拷贝过去，来测试。
//!
//! workspace directory：就是 Cargo.toml 所在的文件夹
//!
//! 运行时机：
//! 1. target 需要重新 build && 符合 build.rs 中设置的条件
//! 2. build.rs 脚本自身发生改变
//!
//! 调试方法: 只推荐在 examples 中直接运行，其他方式都不好使。

fn main()
{
    println!("cargo:rerun-if-changed=shader/glsl/");
    println!("cargo:rerun-if-changed=shader/include/");

    // 设置编译时的环境变量
    let current_dir = std::env::current_dir().unwrap();
    let shader_spv_dir_abs = format!("{}/shader/generate", current_dir.to_str().unwrap());
    println!("cargo:rustc-env=HISS_SHADER_SPV_DIR={shader_spv_dir_abs}");

    // 如果 generate 文件夹不存在，则创建文件夹
    let dst_dir = std::path::Path::new("shader/generate");
    if !dst_dir.is_dir() {
        std::fs::create_dir(dst_dir).expect("fail to mkdir generate");
    }

    // 编译着色器
    compile_all_shader();
}

fn compile_all_shader()
{
    std::fs::read_dir("shader/glsl")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let file_name = entry.file_name().into_string().unwrap();
            if file_name.contains(".vert")
                || file_name.contains(".frag")
                || file_name.contains(".comp")
                || file_name.contains(".rchit")
                || file_name.contains(".rgen")
                || file_name.contains(".rmiss")
            {
                Some(file_name)
            } else {
                None
            }
        })
        .for_each(|shader_file| {
            println!("shader file: {shader_file}");
            std::process::Command::new("glslc")
                .args([
                    "-Ishader/include",
                    "-g",
                    "--target-env=vulkan1.2",
                    "--target-spv=spv1.4", // ray tracing 最低版本为 spv1.4
                    "-o",
                    &format!("shader/generate/{shader_file}.spv"),
                    &format!("shader/glsl/{shader_file}"),
                ])
                .spawn()
                .unwrap_or_else(|_| panic!("failed to compilie shader: {shader_file}"));
        });
}
