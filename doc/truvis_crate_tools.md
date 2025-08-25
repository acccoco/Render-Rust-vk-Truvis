# truvis-crate-tools

## 概述
为 Truvis 工作区内各个 crate 提供共享工具和实用程序。包含路径管理、资源定位、构建辅助等跨 crate 功能。

## 核心功能

### 资源路径管理
提供工作区相对路径的统一处理：

```rust
use truvis_crate_tools::resource::TruvisPath;

// 获取资产路径（相对于工作区根目录）
let model_path = TruvisPath::assets_path("models/sponza.fbx");
let texture_path = TruvisPath::resources_path("textures/uv_checker.png");
let shader_path = TruvisPath::shader_path("rt/raygen.slang");

// 获取绝对路径
let abs_path = model_path.to_absolute()?;
```

### 工作区检测
- 自动检测当前工作区根目录
- 基于 `Cargo.toml` 的工作区识别
- 跨平台路径处理

### 构建辅助工具
- 构建时的路径解析
- 环境变量管理
- 平台特定的路径处理

## 主要模块

### 路径系统 (`src/resource.rs`)
```rust
// 文件：crates/truvis-crate-tools/src/resource.rs
pub struct TruvisPath {}
impl TruvisPath {
    pub fn assets_path(filename: &str) -> String;
    pub fn resources_path(filename: &str) -> String;
    pub fn shader_path(filename: &str) -> String;
    
    fn workspace_path() -> PathBuf {
        // 从 CARGO_MANIFEST_DIR 推导工作区根目录
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent() // crates/truvis-crate-tools -> crates
            .unwrap()
            .parent() // crates -> workspace root
            .unwrap()
            .to_path_buf()
    }
}
```

### 日志初始化 (`src/init_log.rs`)
- 统一的日志配置
- 环境变量控制日志级别

### 命名数组 (`src/named_array.rs`)
- 类型安全的命名数组实用工具

## 使用场景

### 着色器加载（实际实现）
```rust
// 编译后的着色器路径（注意是 .build 目录）
let vertex_shader = TruvisPath::shader_path("hello_triangle/vert.slang.spv");
let fragment_shader = TruvisPath::shader_path("hello_triangle/frag.slang.spv");
```

### 资产加载（实际路径结构）
```rust
// assets/ 目录下的模型文件
let sponza_model = TruvisPath::assets_path("sponza.fbx");

// resources/ 目录下的纹理和其他资源
let texture = TruvisPath::resources_path("uv_checker.png");
let font = TruvisPath::resources_path("mplus-1p-regular.ttf");
```

## 与其他 crate 的集成

### truvis-render 集成
- 资产路径的统一管理
- 配置文件的加载
- 运行时资源定位

### shader-build 集成
- 着色器源文件的定位
- 输出目录的管理
- 依赖关系追踪

### truvis-cxx 集成
- C++ 库路径的配置
- 动态库的定位
- 构建产物的管理

## 开发辅助

### 调试支持
- 路径解析的调试信息
- 资源丢失的诊断
- 构建路径的验证

### 错误处理
```rust
pub enum PathError {
    WorkspaceNotFound,
    ResourceNotFound(PathBuf),
    InvalidPath(String),
    IoError(std::io::Error),
}
```

### 日志集成
- 路径操作的详细日志
- 资源加载状态追踪
- 性能监控支持

## 配置系统

### 环境变量支持
- `TRUVIS_WORKSPACE_ROOT`: 覆盖工作区根目录
- `TRUVIS_ASSETS_DIR`: 自定义资产目录
- `TRUVIS_DEBUG_PATHS`: 启用路径调试

### 构建时配置
```rust
// build.rs 中的使用
use truvis_crate_tools::build::*;

fn main() {
    let workspace_root = find_workspace_root()?;
    println!("cargo:rustc-env=WORKSPACE_ROOT={}", workspace_root.display());
}
```

## 最佳实践

### 路径管理
- 始终使用 `TruvisPath` 而不是硬编码路径
- 在构建时验证资源存在性
- 使用相对路径保证可移植性

### 错误处理
- 提供清晰的错误信息
- 包含足够的上下文用于调试
- 优雅地处理资源缺失

### 性能考虑
- 缓存路径解析结果
- 延迟加载资源
- 避免重复的文件系统访问

## 未来扩展
- 资源热重载支持
- 资源压缩和打包
- 云端资源同步
- 多语言资源管理
