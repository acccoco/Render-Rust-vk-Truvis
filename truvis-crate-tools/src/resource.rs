use std::{
    env,
    path::{Path, PathBuf},
};

/// 统一资源路径管理
///
/// 所有路径基于工作区根目录（通过 `CARGO_MANIFEST_DIR` 推导）。
/// 避免使用硬编码相对路径，确保在不同构建环境下路径一致。
///
/// # 使用示例
/// ```ignore
/// let model = TruvisPath::assets_path("sponza.fbx");           // assets/sponza.fbx
/// let texture = TruvisPath::resources_path("uv_checker.png");  // resources/uv_checker.png
/// let shader = TruvisPath::shader_path("rt/raygen.slang"); // shader/.build/rt/raygen.slang
/// ```
pub struct TruvisPath {}
// 核心路径
impl TruvisPath {
    /// 获取工作区根目录
    pub fn workspace_path() -> PathBuf {
        // 从当前包的位置推导workspace目录
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    }

    pub fn target_path() -> PathBuf {
        Self::workspace_path().join("target")
    }
}
// 根目录下
impl TruvisPath {
    pub fn engine_path() -> PathBuf {
        Self::workspace_path().join("engine")
    }

    /// 获取 `assets/` 目录下的文件路径
    pub fn assets_path(filename: &str) -> std::path::PathBuf {
        Self::workspace_path().join("assets").join(filename)
    }
    pub fn assets_path_str(filename: &str) -> String {
        Self::assets_path(filename).to_str().unwrap().to_string()
    }

    /// 获取 `resources/` 目录下的文件路径
    pub fn resources_path(filename: &str) -> std::path::PathBuf {
        Self::workspace_path().join("resources").join(filename)
    }
    pub fn resources_path_str(filename: &str) -> String {
        Self::resources_path(filename).to_str().unwrap().to_string()
    }

    pub fn tools_path() -> PathBuf {
        Self::workspace_path().join("tools")
    }
}
// engine 目录下
impl TruvisPath {
    pub fn shader_root_path() -> PathBuf {
        Self::engine_path().join("shader")
    }

    /// 获取 `shader/.build/` 目录下的着色器路径（编译后的 SPIR-V）
    pub fn shader_build_path_str(filename: &str) -> String {
        let shader_path = Self::shader_root_path().join(".build").join(filename);
        let mut shader_build_path = shader_path.to_str().unwrap().to_string();
        shader_build_path.push_str(".spv");
        shader_build_path
    }

    pub fn cxx_root_path() -> PathBuf {
        Self::engine_path().join("cxx")
    }
}
