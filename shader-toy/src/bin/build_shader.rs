/// 将指定目录下的所有 shader 文件编译为 spv 文件，输出到同一目录下
fn compile_one_dir(dir: &std::path::Path) -> anyhow::Result<()>
{
    std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let file_name = entry.as_ref().unwrap().file_name().into_string().unwrap();
            if file_name.ends_with(".vert") ||
                file_name.ends_with(".frag") ||
                file_name.ends_with(".comp") ||
                file_name.ends_with(".rchit") ||
                file_name.ends_with(".rgen") ||
                file_name.ends_with(".rmiss")
            {
                Some(entry.unwrap())
            } else {
                None
            }
        })
        .for_each(|entry| {
            let shader_path = entry.path().to_str().unwrap().to_string();
            let shader_name = entry.file_name().into_string().unwrap();
            let dir = entry.path().parent().unwrap().to_str().unwrap().to_string();
            let output_path = format!("{}/{}.spv", dir, shader_name);
            let output = std::process::Command::new("glslc")
                .args([
                    "-Ishader/include",
                    "-g",
                    "--target-env=vulkan1.2",
                    "--target-spv=spv1.4", // ray tracing 最低版本为 spv1.4
                    "-o",
                    &output_path,
                    &shader_path,
                ])
                .output()
                .unwrap();

            if !output.status.success() {
                println!("stdout: {stdout}", stdout = String::from_utf8_lossy(&output.stdout));
                println!("stderr: {stderr}", stderr = String::from_utf8_lossy(&output.stderr));
                panic!("failed to compilie shader: {:#?}", entry)
            } else {
                println!("compile shader: {:#?}", entry);
            }
        });

    Ok(())
}


fn compile_all_shader() -> anyhow::Result<()>
{
    std::fs::read_dir("shader")?
        .filter(|entry| entry.as_ref().unwrap().path().is_dir())
        .for_each(|entry| compile_one_dir(&entry.unwrap().path()).unwrap());

    Ok(())
}


fn main() -> anyhow::Result<()>
{
    compile_all_shader()
}
