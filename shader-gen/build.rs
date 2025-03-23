use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use tera::Tera;

#[derive(Deserialize, Serialize)]
struct ShaderLayout {
    name: String,
    bindings: Vec<Binding>,
}

#[derive(Deserialize, Serialize)]
struct Binding {
    name: String,
    binding: u32,
    type_: String,
    rust_type: Option<String>,
    hlsl_type: Option<String>,
}

fn main() -> Result<()> {
    // 初始化模板引擎
    let tera = Tera::new("templates/**/tera.*")?;

    // 读取 YAML 定义
    let yaml_content = fs::read_to_string("definitions/shader_layouts.yaml")?;
    let layouts: Vec<ShaderLayout> = serde_yaml::from_str(&yaml_content)?;

    // 确保输出目录存在
    fs::create_dir_all("generated/rust")?;
    fs::create_dir_all("generated/hlsl")?;

    // 为每个布局生成代码
    for layout in layouts {
        // 生成 Rust 代码
        let rust_code = tera.render("tera.rs", &tera::Context::from_serialize(&layout)?)?;
        fs::write(format!("src/gen/{}.rs", layout.name.to_lowercase()), rust_code)?;

        // 生成 HLSL 代码
        let hlsl_code = tera.render("tera.hlsl", &tera::Context::from_serialize(&layout)?)?;
        fs::write(format!("generated/hlsl/{}.hlsl", layout.name.to_lowercase()), hlsl_code)?;
    }

    // 通知 cargo 重新构建
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=definitions");

    Ok(())
}
