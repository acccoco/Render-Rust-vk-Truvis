//! 将指定目录下的所有 shader 文件编译为 spv 文件，输出到同一目录下

use anyhow::{bail, Error, Result};

#[derive(Debug)]
enum ShaderType
{
    Vertex,
    Fragment,
    Compute,
    RayClosestHit,
    RayGen,
    RayMiss,
}

#[derive(Debug)]
struct Shader
{
    shader_path: std::path::PathBuf,
    shader_type: ShaderType,
    output_path: std::path::PathBuf,
}

impl Shader
{
    fn from_dir_entry(entry: &std::fs::DirEntry) -> Option<Self>
    {
        let shader_path = entry.path();
        let shader_name = entry.file_name().into_string().unwrap();
        let dir = shader_path.parent().unwrap().to_str().unwrap().to_string();
        let output_path = format!("{}/{}.spv", dir, shader_name);

        let shader_type = if shader_name.ends_with(".vert") {
            ShaderType::Vertex
        } else if shader_name.ends_with(".frag") {
            ShaderType::Fragment
        } else if shader_name.ends_with(".comp") {
            ShaderType::Compute
        } else if shader_name.ends_with(".rchit") {
            ShaderType::RayClosestHit
        } else if shader_name.ends_with(".rgen") {
            ShaderType::RayGen
        } else if shader_name.ends_with(".rmiss") {
            ShaderType::RayMiss
        } else {
            return None;
        };

        Some(Self {
            shader_path,
            shader_type,
            output_path: std::path::PathBuf::from(output_path),
        })
    }

    fn compile(&self) -> anyhow::Result<()>
    {
        let output = std::process::Command::new("glslc")
            .args([
                "-Ishader/include",
                "-g",
                "--target-env=vulkan1.2",
                "--target-spv=spv1.4", // ray tracing 最低版本为 spv1.4
                "-o",
                self.output_path.to_str().unwrap(),
                self.shader_path.to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            println!("stdout: {stdout}", stdout = String::from_utf8_lossy(&output.stdout));
            println!("stderr: {stderr}", stderr = String::from_utf8_lossy(&output.stderr));
            bail!("failed to compilie shader: {:#?}", self.shader_path);
        }
        Ok(())
    }
}


fn compile_one_dir(dir: &std::path::Path) -> anyhow::Result<()>
{
    std::fs::read_dir(dir)?
        .filter_map(|entry| Shader::from_dir_entry(entry.as_ref().unwrap()))
        .for_each(|entry| {
            println!("compile shader: {:#?}", entry);
            entry.compile().unwrap()
        });

    Ok(())
}


fn compile_all_shader() -> anyhow::Result<()>
{
    std::fs::read_dir("shader")?
        .filter(|entry| entry.as_ref().unwrap().path().is_dir())
        .for_each(|entry| {
            println!("compile shader in dir: {:#?}", entry.as_ref().unwrap().path());
            compile_one_dir(&entry.unwrap().path()).unwrap()
        });

    Ok(())
}


fn main() -> anyhow::Result<()>
{
    compile_all_shader()
}
