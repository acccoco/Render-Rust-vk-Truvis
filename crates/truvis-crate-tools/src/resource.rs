use std::env;
use std::path::{Path, PathBuf};

pub struct TruvisPath {}
impl TruvisPath {
    pub fn assets_path(filename: &str) -> String {
        let workspace_dir = Self::workspace_path();
        let assets_path = workspace_dir.join("assets").join(filename);
        assets_path.to_str().unwrap().to_string()
    }

    pub fn resources_path(filename: &str) -> String {
        let workspace_dir = Self::workspace_path();
        let resources_path = workspace_dir.join("resources").join(filename);
        resources_path.to_str().unwrap().to_string()
    }

    pub fn shader_path(filename: &str) -> String {
        let workspace_dir = Self::workspace_path();
        let shader_path = workspace_dir.join("shader").join(".build").join(filename);
        shader_path.to_str().unwrap().to_string()
    }

    fn workspace_path() -> PathBuf {
        // 从当前包的位置推导workspace目录
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent() // 从 crates/truvis-crate-tools 到 crates
            .unwrap()
            .parent() // 从 crates 到 workspace root
            .unwrap()
            .to_path_buf()
    }
}
