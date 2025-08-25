# shader-build

## 概述
手动编译 Slang 着色器的构建工具，支持多种着色器格式和目标平台。提供批量编译、依赖追踪和错误报告功能。

## 架构组织

### 编译器接口 (`src/compiler.rs`)
- Slang 编译器的 Rust 封装
- 多目标编译支持（SPIRV, HLSL, GLSL）
- 编译参数和优化选项管理

### 构建管理 (`src/build_manager.rs`)
- 项目级别的着色器构建管理
- 增量编译和依赖追踪
- 并行编译支持

### 文件监控 (`src/watcher.rs`)
- 着色器文件变更监控
- 自动重新编译
- 热重载支持

## 核心功能

### 批量编译
```bash
# 编译所有着色器
cargo run --bin build_shader

# 编译特定目录
cargo run --bin build_shader -- --dir shader/src/rt

# 增量编译
cargo run --bin build_shader -- --incremental
```

### 支持的着色器格式
- **输入**: Slang (.slang) 着色器源码
- **输出**: SPIR-V (.spirv) 二进制文件
- **中间**: HLSL, GLSL 代码生成（可选）

### 多目标编译
```rust
pub enum CompileTarget {
    Spirv,          // Vulkan SPIR-V
    Hlsl,           // Direct3D HLSL
    Glsl,           // OpenGL GLSL
    MetalSL,        // Metal Shading Language
}
```

## 编译流程

### 源文件发现
- 自动扫描 `shader/src/` 目录
- 递归查找 `.slang` 文件
- 跳过特定的排除模式

### 依赖解析
```rust
// 解析 #include 依赖
let dependencies = parse_includes(&shader_source)?;
for dep in dependencies {
    if is_newer(&dep, &output_file) {
        needs_recompile = true;
        break;
    }
}
```

### 编译执行
```rust
pub struct CompileJob {
    pub source_file: PathBuf,
    pub target: CompileTarget,
    pub optimization: OptimizationLevel,
    pub debug_info: bool,
}

impl CompileJob {
    pub fn execute(&self) -> CompileResult {
        // 调用 Slang 编译器
        // 处理编译输出
        // 生成目标文件
    }
}
```

## 使用模式

### 开发时编译
```bash
# 初次构建时编译所有着色器
cargo run --bin build_shader

# 开发过程中的增量编译
cargo run --bin build_shader -- --watch
```

### 集成到构建系统
```rust
// 在 build.rs 中
fn main() {
    let shader_dir = "shader/src";
    let output_dir = "shader/.build";
    
    ShaderBuilder::new()
        .source_dir(shader_dir)
        .output_dir(output_dir)
        .target(CompileTarget::Spirv)
        .build_all()?;
}
```

### 运行时加载
```rust
use truvis_crate_tools::TruvisPath;

// 加载编译后的着色器
let vertex_spirv = TruvisPath::shader_build_path("hello_triangle/vert.spirv");
let fragment_spirv = TruvisPath::shader_build_path("hello_triangle/frag.spirv");

let vertex_module = ShaderModule::from_spirv(&device, &vertex_spirv)?;
let fragment_module = ShaderModule::from_spirv(&device, &fragment_spirv)?;
```

## 编译配置

### 优化级别
```rust
pub enum OptimizationLevel {
    None,           // 无优化，快速编译
    Size,           // 优化二进制大小
    Performance,    // 优化运行性能
    Debug,          // 调试友好
}
```

### 编译选项
```rust
pub struct CompileOptions {
    pub target: CompileTarget,
    pub optimization: OptimizationLevel,
    pub debug_info: bool,
    pub warnings_as_errors: bool,
    pub include_paths: Vec<PathBuf>,
    pub defines: HashMap<String, String>,
}
```

### 配置文件支持
```json
{
    "targets": ["spirv"],
    "optimization": "performance",
    "debug_info": true,
    "include_paths": ["shader/include"],
    "defines": {
        "MAX_LIGHTS": "64",
        "ENABLE_PBR": "1"
    }
}
```

## 错误处理和报告

### 详细错误信息
```rust
pub struct CompileError {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub severity: ErrorSeverity,
}

pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}
```

### 错误报告格式
```
Error in shader/src/rt/raygen.slang:23:15
  |
23|     float3 color = sample_texture(invalid_id);
  |                   ^^^^^^^^^^^^^^^ undefined function
  |
  = help: Did you forget to include 'sampling.slangi'?
```

### 批量错误处理
- 收集所有编译错误
- 按文件和严重程度分类
- 提供修复建议

## 性能优化

### 并行编译
```rust
use rayon::prelude::*;

compile_jobs.par_iter()
    .map(|job| job.execute())
    .collect::<Result<Vec<_>, _>>()?;
```

### 增量编译
- 基于文件修改时间的依赖检查
- 缓存编译结果
- 跳过未变更的文件

### 编译缓存
- 基于源文件哈希的缓存
- 跨构建会话的持久化缓存
- 缓存失效和清理

## 集成和部署

### CI/CD 集成
```yaml
# GitHub Actions 示例
- name: Build Shaders
  run: cargo run --bin build_shader --release
  
- name: Verify Shader Compilation
  run: |
    if [ ! -d "shader/.build" ]; then
      echo "Shader compilation failed"
      exit 1
    fi
```

### 发布准备
- 验证所有着色器编译成功
- 检查二进制文件完整性
- 生成着色器清单

## 开发工具

### 着色器依赖图
- 可视化着色器间的依赖关系
- 识别循环依赖
- 优化编译顺序

### 性能分析
- 编译时间统计
- 内存使用监控
- 瓶颈识别

### 调试辅助
- 源码映射保留
- 调试符号生成
- GPU 调试器集成

## 未来扩展
- 更多目标平台支持
- 着色器变体系统
- 自动性能优化
- 着色器热重载支持
