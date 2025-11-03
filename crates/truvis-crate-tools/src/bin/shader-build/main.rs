//! 将指定目录下的所有 shader 文件编译为 spv 文件，输出到同一目录下

mod glsl;
mod hlsl;
mod shader_build;
mod slang;

use rayon::prelude::*;
use shader_build::EnvPath;
use truvis_crate_tools::init_log::init_log;

/// shader 的 stage
#[derive(Debug)]
enum ShaderStage {
    Vertex,

    /// hlsl Hull shader
    _TessellationControl,

    /// hlsl Domain shader
    _TessellationEvaluation,
    _Geometry,

    /// hlsl Pixel shader
    Fragment,
    Compute,

    RayGen,
    _AnyHit,
    ClosestHit,
    Miss,
    _Intersection,
    _RayCallable,

    /// hlsl Amplification shader
    _Task,
    _Mesh,

    /// slang 不需要明确的 shader stage
    General,
}

/// shader 编译器类型
#[derive(Debug)]
enum ShaderCompilerType {
    Glsl,
    Hlsl,
    Slang,
}

/// 一个具体的编译任务
#[derive(Debug)]
struct ShaderCompileTask {
    shader_path: std::path::PathBuf,
    output_path: std::path::PathBuf,
    shader_stage: ShaderStage,
    shader_type: ShaderCompilerType,
}

impl ShaderCompileTask {
    /// 生成 shader compile 的任务
    ///
    /// entry 是相对于 workspace 的
    fn new(entry: &walkdir::DirEntry) -> Option<Self> {
        let shader_path = entry.path().to_str()?.replace("\\", "/");
        let shader_path = std::path::Path::new(&shader_path);
        // 相对于 shader 的路径
        let relative_path = shader_path.strip_prefix(EnvPath::shader_src_path()).unwrap();
        let shader_name = entry.file_name().to_str().unwrap();

        let mut output_path = EnvPath::shader_build_path().join(relative_path);
        output_path.set_extension("spv");
        let shader_stage = Self::get_shader_stage(shader_name)?;
        let shader_type = Self::select_shader_compiler(shader_name);

        Some(Self {
            shader_path: shader_path.to_path_buf(),
            shader_type,
            shader_stage,
            output_path,
        })
    }

    /// 根据 shader 文件名获取 shader stage
    fn get_shader_stage(shader_name: &str) -> Option<ShaderStage> {
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
        } else if shader_name.ends_with(".slang") {
            ShaderStage::General
        } else {
            return None;
        };

        Some(shader_stage)
    }

    /// 根据 shader 文件名选择编译器
    fn select_shader_compiler(shader_name: &str) -> ShaderCompilerType {
        if shader_name.ends_with(".hlsl") {
            ShaderCompilerType::Hlsl
        } else if shader_name.ends_with(".slang") {
            ShaderCompilerType::Slang
        } else {
            ShaderCompilerType::Glsl
        }
    }

    /// 根据 cmd 执行的结果，处理输出信息
    fn process_cmd_output(&self, output: std::process::Output) {
        if !output.stdout.is_empty() {
            log::info!("stdout: {stdout}", stdout = String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            log::error!("stderr: {stderr}", stderr = String::from_utf8_lossy(&output.stderr));
        }
    }

    fn build(&self) {
        std::fs::create_dir_all(self.output_path.parent().unwrap()).unwrap();
        match self.shader_type {
            ShaderCompilerType::Glsl => self.build_glsl(),
            ShaderCompilerType::Hlsl => self.build_hlsl(),
            ShaderCompilerType::Slang => self.build_slang(),
        }
    }

    /// 使用 glslc 编译 glsl 文件
    fn build_glsl(&self) {
        let output = std::process::Command::new("glslc")
            .args([
                &format!("-I{:?}", EnvPath::shader_include_path()),
                "-g",
                "--target-env=vulkan1.2",
                "--target-spv=spv1.4", // ray tracing 最低版本为 spv1.4
                "-o",
                self.output_path.to_str().unwrap(),
                self.shader_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        self.process_cmd_output(output);
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
        // dxc.exe -spirv -T vs_6_1 -E main .\input.vert -Fo .\output.vert.spv
        // -fspv-extension=SPV_EXT_descriptor_indexing
        let shader_stage_tag = match self.shader_stage {
            ShaderStage::Vertex => "vs",
            ShaderStage::_TessellationControl => "hs",
            ShaderStage::_TessellationEvaluation => "ds",
            ShaderStage::_Geometry => "gs",
            ShaderStage::Fragment => "ps",
            ShaderStage::Compute => "cs",
            ShaderStage::RayGen
            | ShaderStage::_AnyHit
            | ShaderStage::ClosestHit
            | ShaderStage::Miss
            | ShaderStage::_Intersection
            | ShaderStage::_RayCallable => "lib",
            ShaderStage::_Task => "as",
            ShaderStage::_Mesh => "ms",
            ShaderStage::General => panic!("dxc does not support slang"),
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
        self.process_cmd_output(output);
    }

    /// 使用 slangc 编译 slang 文件
    fn build_slang(&self) {
        let output = std::process::Command::new(EnvPath::slangc_path())
            .args([
                "-I",
                EnvPath::shader_include_path().to_str().unwrap(),
                "-g2",                         // 生成 debug info 默认是 g2
                "-matrix-layout-column-major", // 列主序
                "-fvk-use-entrypoint-name",    // 具有多个 entry 时，需要这个选项
                "-target",
                "spirv",
                "-o",
                self.output_path.to_str().unwrap(),
                self.shader_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        self.process_cmd_output(output);
    }
}

fn main() {
    init_log();

    log::info!("shader include path: {:?}", EnvPath::shader_include_path());
    log::info!("shader src: {:?}", EnvPath::shader_src_path());
    log::info!("shader build output: {:?}", EnvPath::shader_build_path());

    // 编译 shader 目录下的所有 shader 文件
    // 假定嵌套深度为 1
    // 以下 entry 都是相对于 workspace 的
    walkdir::WalkDir::new(EnvPath::shader_src_path())
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_file())
        .filter_map(|dir| ShaderCompileTask::new(&dir))
        .par_bridge() // 并行化
        .for_each(|entry| {
            log::info!("compile shader: {:?}", entry.shader_path);
            entry.build();
        });
}
