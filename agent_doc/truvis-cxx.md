# truvis-cxx

## 概述
为 Truvis 渲染引擎提供 C++ 库集成，主要是 Assimp 模型加载库的 Rust 绑定。通过 CMake 构建系统自动编译 C++ 依赖并生成 Rust FFI 绑定。

## 架构组织

### C++ 源码 (`cxx/`)
- Assimp 库的集成和配置
- C++ 到 C FFI 的适配层
- 内存管理和错误处理

### 构建系统 (`build.rs`)
- CMake 自动构建配置
- bindgen 自动生成 Rust 绑定
- 动态库的自动复制和部署

### 输出目录 (`cargo-cmake-output/`)
- CMake 构建的中间产物
- 编译生成的静态库和动态库
- 调试符号和构建日志

## 核心功能

### Assimp 集成
- 支持主流 3D 模型格式（FBX, OBJ, GLTF, DAE 等）
- 网格数据的提取和转换
- 材质和纹理信息的解析
- 场景层次结构的处理

### 内存管理
- C++ 对象的安全生命周期管理
- Rust 和 C++ 之间的零拷贝数据传输
- 异常安全的资源清理

### 数据转换
- Assimp 数据结构到 `model-manager` 类型的转换
- 坐标系统的统一处理
- 顶点数据的重新组织和优化

## FFI 绑定

### 自动生成
```rust
// build.rs 中的 bindgen 配置
let bindings = bindgen::Builder::new()
    .header("cxx/assimp_wrapper.h")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks))
    .generate()
    .expect("Unable to generate bindings");
```

### 安全封装
```rust
pub struct AssimpScene {
    ptr: *mut c_void,
}

impl AssimpScene {
    pub fn load_from_file(path: &str) -> Result<Self> {
        // 安全的 FFI 调用
    }
    
    pub fn extract_meshes(&self) -> Vec<Mesh> {
        // C++ 数据到 Rust 的转换
    }
}

impl Drop for AssimpScene {
    fn drop(&mut self) {
        // 自动清理 C++ 资源
    }
}
```

## 构建配置

### CMake 集成
```cmake
# CMakeLists.txt 配置
find_package(assimp REQUIRED)
target_link_libraries(truvis_cxx_native assimp::assimp)
```

### Cargo 配置
```toml
[build-dependencies]
cmake = { workspace = true }
bindgen = { workspace = true }
```

## 使用模式

### 模型加载
```rust
use truvis_cxx::AssetLoader;

let scene = AssetLoader::load_scene("assets/models/sponza.fbx")?;
let meshes = scene.extract_meshes();
let materials = scene.extract_materials();

// 转换为引擎内部格式
for mesh in meshes {
    let geometry = mesh.to_geometry()?;
    let material = mesh.to_material()?;
    // ... 添加到场景中
}
```

### 材质处理
```rust
let material_data = scene.get_material(material_index)?;
let diffuse_texture = material_data.diffuse_texture_path();
let normal_texture = material_data.normal_texture_path();
// ... 加载纹理
```

## 部署和分发

### 动态库管理
- 自动将必要的 DLL 复制到 `target/debug/` 和 `target/release/`
- 支持不同平台的动态库命名约定
- 运行时依赖的自动解析

### 版本兼容性
- 特定版本的 Assimp 库绑定
- 向后兼容性的保证
- ABI 稳定性考虑

## 平台支持
- **Windows**: Visual Studio 2019+ 编译器
- **Linux**: GCC 或 Clang 编译器
- **macOS**: Apple Clang 编译器

## 开发注意事项

### 内存安全
- 所有 C++ 指针都被安全封装
- 使用 RAII 模式管理资源生命周期
- 防止悬垂指针和内存泄漏

### 错误处理
- C++ 异常到 Rust Result 的转换
- 详细的错误信息传递
- 优雅的失败处理

### 性能优化
- 最小化跨 FFI 边界的数据拷贝
- 批量数据传输
- 并行处理支持

## 未来扩展
- 更多 C++ 图形库的集成
- 物理引擎绑定
- 音频处理库集成
