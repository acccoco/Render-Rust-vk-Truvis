use std::sync::OnceLock;
use truvis_crate_tools::resource::TruvisPath;

/// 当前项目的环境路径，基于 workspace 根目录
pub struct EnvPath;
impl EnvPath {
    /// shader src 的路径
    pub fn shader_src_path() -> &'static std::path::Path {
        static P: OnceLock<std::path::PathBuf> = OnceLock::new();
        P.get_or_init(|| {
            let mut p = TruvisPath::workspace_path();
            p.extend(["shader", "src"]);
            p
        })
    }

    /// 编译 shader 的输出路径
    pub fn shader_build_path() -> &'static std::path::Path {
        static P: OnceLock<std::path::PathBuf> = OnceLock::new();
        P.get_or_init(|| {
            let mut p = TruvisPath::workspace_path();
            p.extend(["shader", ".build"]);
            p
        })
    }

    /// shader 的 include 目录
    pub fn shader_include_path() -> &'static std::path::Path {
        static P: OnceLock<std::path::PathBuf> = OnceLock::new();
        P.get_or_init(|| {
            let mut p = TruvisPath::workspace_path();
            p.extend(["shader", "include"]);
            p
        })
    }

    /// slang shader 编译器的路径
    pub fn slangc_path() -> &'static std::path::Path {
        static P: OnceLock<std::path::PathBuf> = OnceLock::new();
        P.get_or_init(|| {
            let mut p = TruvisPath::workspace_path();
            p.extend(["tools", "slang", "bin", "slangc.exe"]);
            p
        })
    }
}
