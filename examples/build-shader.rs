//! 用于测试 build.rs 的。
//!
//! 由于编译脚本的输出不明显，因此

fn main()
{
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
