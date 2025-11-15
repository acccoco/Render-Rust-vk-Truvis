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
/// let shader = TruvisPath::shader_path("rt/raygen.slang.spv"); // shader/.build/rt/raygen.slang.spv
/// ```
pub struct TruvisPath {}
impl TruvisPath {
    /// 获取 `assets/` 目录下的文件路径
    pub fn assets_path(filename: &str) -> String {
        let workspace_dir = Self::workspace_path();
        let assets_path = workspace_dir.join("assets").join(filename);
        assets_path.to_str().unwrap().to_string()
    }

    /// 获取 `resources/` 目录下的文件路径
    pub fn resources_path(filename: &str) -> String {
        let workspace_dir = Self::workspace_path();
        let resources_path = workspace_dir.join("resources").join(filename);
        resources_path.to_str().unwrap().to_string()
    }

    /// 获取 `shader/.build/` 目录下的着色器路径（编译后的 SPIR-V）
    pub fn shader_path(filename: &str) -> String {
        let workspace_dir = Self::workspace_path();
        let shader_path = workspace_dir.join("shader").join(".build").join(filename);
        shader_path.to_str().unwrap().to_string()
    }

    /// 获取工作区根目录
    pub fn workspace_path() -> PathBuf {
        // 从当前包的位置推导workspace目录
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent() // 从 crates/truvis-crate-tools 到 crates
            .unwrap()
            .parent() // 从 crates 到 workspace root
            .unwrap()
            .to_path_buf()
    }
}
