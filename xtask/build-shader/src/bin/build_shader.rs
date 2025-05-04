//! 将指定目录下的所有 shader 文件编译为 spv 文件，输出到同一目录下

use std::fs;

#[derive(Debug)]
enum ShaderStage {
    Vertex,

    /// hlsl Hull shader
    TessellationControl,

    /// hlsl Domain shader
    TessellationEvaluation,
    Geometry,

    /// hlsl Pixel shader
    Fragment,
    Compute,

    RayGen,
    AnyHit,
    ClosestHit,
    Miss,
    Intersection,
    RayCallable,

    /// hlsl Amplification shader
    Task,
    Mesh,
}

#[derive(Debug)]
enum ShaderType {
    Glsl,
    Hlsl,
}

#[derive(Debug)]
struct ShaderCompileEntry {
    shader_path: std::path::PathBuf,
    shader_stage: ShaderStage,
    output_path: std::path::PathBuf,
    shader_type: ShaderType,
}

impl ShaderCompileEntry {
    /// 生成 shader compile 的任务
    fn new(entry: &std::fs::DirEntry) -> Option<Self> {
        let shader_path = entry.path();
        let shader_name = entry.file_name().into_string().unwrap();
        let dir = shader_path.parent().unwrap().to_str().unwrap().to_string();
        let output_path = format!("{}/{}.spv", dir, shader_name);

        let shader_stage = if shader_name.ends_with(".vert") || shader_name.ends_with(".vs.hlsl") {
            ShaderStage::Vertex
        } else if shader_name.ends_with(".frag") || shader_name.ends_with(".ps.hlsl") {
            ShaderStage::Fragment
        } else if shader_name.ends_with(".comp") {
            ShaderStage::Compute
        } else if shader_name.ends_with(".rchit") {
            ShaderStage::ClosestHit
        } else if shader_name.ends_with(".rgen") {
            ShaderStage::RayGen
        } else if shader_name.ends_with(".rmiss") {
            ShaderStage::Miss
        } else {
            return None;
        };

        let shader_type = if shader_name.ends_with(".hlsl") { ShaderType::Hlsl } else { ShaderType::Glsl };

        Some(Self {
            shader_path,
            shader_type,
            shader_stage,
            output_path: std::path::PathBuf::from(output_path),
        })
    }

    /// 使用 glslc 编译 glsl 文件
    fn build_glsl(&self) {
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
            .output()
            .unwrap();

        if !output.status.success() {
            if !output.stdout.is_empty() {
                log::info!("stdout: {stdout}", stdout = String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                log::error!("stderr: {stderr}", stderr = String::from_utf8_lossy(&output.stderr));
            }
            panic!("failed to compilie shader: {:#?}", self.shader_path);
        }
    }

    /// 使用 dxc 编译 hlsl 文件，dxc 在 vulkan sdk 中附带
    ///
    /// ref: https://docs.vulkan.org/guide/latest/hlsl.html
    ///
    /// Nsight 使用时：
    ///
    /// https://docs.nvidia.com/nsight-graphics/UserGuide/index.html#configuring-your-application-shaders
    fn build_hlsl(&self) {
        // shader model 6.3 支持 ray tracing
        // shader model 6.5 支持 task shader 和 mesh shader
        // dxc.exe -spirv -T vs_6_1 -E main .\input.vert -Fo .\output.vert.spv -fspv-extension=SPV_EXT_descriptor_indexing
        let shader_stage_tag = match self.shader_stage {
            ShaderStage::Vertex => "vs",
            ShaderStage::TessellationControl => "hs",
            ShaderStage::TessellationEvaluation => "ds",
            ShaderStage::Geometry => "gs",
            ShaderStage::Fragment => "ps",
            ShaderStage::Compute => "cs",
            ShaderStage::RayGen
            | ShaderStage::AnyHit
            | ShaderStage::ClosestHit
            | ShaderStage::Miss
            | ShaderStage::Intersection
            | ShaderStage::RayCallable => "lib",
            ShaderStage::Task => "as",
            ShaderStage::Mesh => "ms",
        };
        let shader_model = "6_7";
        let entry_point = "main";
        let mut cmd = std::process::Command::new("dxc");
        cmd.arg("-spirv")
            .args(["-T", format!("{}_{}", shader_stage_tag, shader_model).as_str()])
            // .arg("-Zpc") // col-major
            .args(["-E", entry_point])
            .arg(self.shader_path.as_os_str())
            .arg("-Fo")
            .arg(self.output_path.as_os_str())
            .arg("-fspv-debug=vulkan-with-source") // SPIR-V NonSemantic Shader DebugInfo Instructions，用于 Nsight 调试
            .arg("-Zi"); // 包含 debug 信息
        let output = cmd.output().unwrap();
        if !output.status.success() {
            if !output.stderr.is_empty() {
                log::info!("stdout: \n{stdout}", stdout = String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                log::error!("stderr: \n{stderr}", stderr = String::from_utf8_lossy(&output.stderr));
            }
            panic!("failed to compilie shader: {:#?}", self.shader_path);
        }
    }
}

/// 编译一个文件夹中的 shader
fn compile_one_dir(dir: &std::path::Path) {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| ShaderCompileEntry::new(entry.as_ref().unwrap())) //
        .filter(|entry| {
            if !entry.output_path.exists() {
                return true;
            }
            let need_re_compile = fs::metadata(&entry.shader_path).unwrap().modified().unwrap()
                > fs::metadata(&entry.output_path).unwrap().modified().unwrap();
            if !need_re_compile {
                log::info!("skip compile shader: {:?}", entry.shader_path);
            }
            need_re_compile
        })
        .for_each(|entry| {
            log::info!("compile shader: {:#?}", entry);
            match entry.shader_type {
                ShaderType::Glsl => entry.build_glsl(),
                ShaderType::Hlsl => entry.build_hlsl(),
            }
        });
}

/// 编译 shader 文件夹下的所有 shader
fn compile_all_shader() {
    std::fs::read_dir("shader")
        .unwrap() //
        .filter(|entry| entry.as_ref().unwrap().path().is_dir())
        .for_each(|entry| {
            log::info!("compile shader in dir: {:#?}", entry.as_ref().unwrap().path());
            compile_one_dir(&entry.unwrap().path())
        });
}

fn main() {
    use simplelog::*;
    TermLogger::init(LevelFilter::Info, ConfigBuilder::new().build(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    compile_all_shader()
}
