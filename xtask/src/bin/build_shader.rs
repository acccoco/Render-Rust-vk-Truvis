//! 将指定目录下的所有 shader 文件编译为 spv 文件，输出到同一目录下

use std::env::args;

use anyhow::{bail, Error, Result};

#[derive(Debug)]
enum ShaderType
{
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,

    RayGen,
    AnyHit,
    ClosestHit,
    Miss,
    Intersection,
    RayCallable,

    Task,
    Mesh,
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
    fn glsl_shader_suffix(shader_type: &ShaderType) -> &'static str
    {
        match shader_type {
            ShaderType::Vertex => ".vert",
            ShaderType::Fragment => ".frag",
            ShaderType::Compute => ".comp",
            ShaderType::ClosestHit => ".rchit",
            ShaderType::RayGen => ".rgen",
            ShaderType::Miss => ".rmiss",
            _ => "",
        }
    }

    /// 传递给 dxc 的，标记 shader stage 的字符串
    fn hlsl_shader_stage_flag(shader_type: ShaderType) -> &'static str
    {
        match shader_type {
            ShaderType::Vertex => "vs",
            ShaderType::TessellationControl => "hs",
            ShaderType::TessellationEvaluation => "ds",
            ShaderType::Geometry => "gs",
            ShaderType::Fragment => "ps",
            ShaderType::Compute => "cs",

            ShaderType::RayGen |
            ShaderType::AnyHit |
            ShaderType::ClosestHit |
            ShaderType::Miss |
            ShaderType::Intersection |
            ShaderType::RayCallable => "lib",

            ShaderType::Task => "as",
            ShaderType::Mesh => "ms",
        }
    }

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
            ShaderType::ClosestHit
        } else if shader_name.ends_with(".rgen") {
            ShaderType::RayGen
        } else if shader_name.ends_with(".rmiss") {
            ShaderType::Miss
        } else {
            return None;
        };

        Some(Self {
            shader_path,
            shader_type,
            output_path: std::path::PathBuf::from(output_path),
        })
    }

    /// 使用 glslc 编译 glsl 文件
    fn build_glsl(&self) -> anyhow::Result<()>
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

    /// 使用 dxc 编译 hlsl 文件，dxc 在 vulkan sdk 中附带
    fn build_hlsl(&self) -> anyhow::Result<()>
    {
        // dxc.exe -spirv -T vs_6_1 -E main .\input.vert -Fo .\output.vert.spv -fspv-extension=SPV_EXT_descriptor_indexing

        let output = std::process::Command::new("dxc")
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

        Ok(())
    }

    /// see: https://docs.vulkan.org/guide/latest/hlsl.html
    fn dxc_wrapper(&self) -> std::process::Command
    {
        let shader_stage_tag = match self.shader_type {
            ShaderType::Vertex => "vs",
            ShaderType::Fragment => "ps",
            _ => "lib",
        };
        let shader_model = "6_7";
        let entry_point = "main";
        let mut cmd = std::process::Command::new("dxc");
        cmd.arg("-spirv")
            .arg("-T")
            .arg(format!("{}_{}", shader_stage_tag, shader_model))
            .arg("-E")
            .arg(entry_point)
            .arg(self.shader_path.as_os_str())
            .arg("-Fo")
            .arg(self.output_path.as_os_str());
        cmd
    }
}


fn compile_one_dir(dir: &std::path::Path) -> anyhow::Result<()>
{
    std::fs::read_dir(dir)?
        .filter_map(|entry| Shader::from_dir_entry(entry.as_ref().unwrap()))
        .for_each(|entry| {
            println!("compile shader: {:#?}", entry);
            entry.build_glsl().unwrap()
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