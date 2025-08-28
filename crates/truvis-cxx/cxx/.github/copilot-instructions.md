# Truvis C++ 场景加载器 - AI 编程指南

## 项目概述
这是一个 C++ 动态链接库 (`truvis-assimp-cxx.dll`)，为基于 Assimp 的 3D 场景加载提供 C FFI 绑定。它是基于 Rust 的 Truvis 渲染系统的一部分，旨在连接 C++ 3D 资产处理与 Rust 图形代码。

## 架构设计

### 核心组件
- **FFI 层**: `lib.hpp`/`lib.cpp` - 与 Rust 集成的 C 兼容接口
- **场景处理**: `scene_loader/` - 基于 Assimp 的 3D 场景加载和转换
- **数据结构**: `c_data_define.hpp` - 具有严格对齐的 C 兼容结构体

### 关键设计模式
- **C FFI 接口**: 所有公共 API 使用 `extern "C"` 和 void* 不透明指针
- **仅移动的 RAII 类型**: Geometry 和 Instance 结构体禁止拷贝但允许移动
- **右手坐标系**: X-Right, Y-Up 约定，带有明确的文档说明
- **内存管理**: C 数组手动分配，析构函数自动清理

## 开发工作流

### 构建系统
```bash
# Windows 使用 Visual Studio（主要方式）
cd build
cmake -G "Visual Studio 17 2022" -A x64 -DCMAKE_CONFIGURATION_TYPES="Debug;Release" ..

# 备选方案：Clang-cl + Ninja
cmake -DCMAKE_BUILD_TYPE=Debug -DCMAKE_MAKE_PROGRAM=ninja.exe \
  -DCMAKE_C_COMPILER=clang-cl.exe -DCMAKE_CXX_COMPILER=clang-cl.exe ..
```

### 依赖项
- **vcpkg**: 包管理器，使用 `vcpkg.json` 清单文件
- **Assimp 5.4.3**: 3D 资产导入库（锁定版本）
- **GLM 1.0.1**: 向量/矩阵数学库

### 测试
```bash
# 使用测试场景文件运行
./Debug/main.exe path/to/scene.obj
```

## 编码约定

### DLL 导出模式
```cpp
#ifdef BUILDING_DLL    // 由 CMake 设置
#define DLL_API __declspec(dllexport)
#else
#define DLL_API __declspec(dllimport)
#endif
```

### C 兼容数据结构
- 使用 `alignas(4)` 确保一致的内存布局
- 固定大小的 C 数组：`char name[PATH_BUFFER_SIZE]` 而非 std::string
- 明确的坐标系文档："坐标系：右手系，X-Right，Y-Up"

### 内存安全
- `CxxRasterGeometry` 和 `CxxInstance` 是仅移动类型，带有自定义析构函数
- C API 函数对所有 void* 参数进行空指针检查
- 通过 C++ 层的 RAII 进行资源清理，从 C 侧手动调用 free_scene()

## 文件组织

### 公共头文件 (`include/public/`)
- `c_data_define.hpp`: FFI 边界的 C 兼容结构体

### 私有头文件 (`include/private/`)
- `scene_loader.hpp`: 内部 C++ 实现细节
- `data_convert.hpp`: Assimp 到内部格式的转换

### 源码结构
- `lib.cpp`: FFI 包装函数
- `scene_loader/`: Assimp 集成和数据处理

## 集成要点

### Rust FFI 期望
- 所有公共函数返回基本类型或不透明指针
- 通过空返回值处理错误，而非异常
- 内存由 C++ 侧管理，提供显式释放函数

### Assimp 处理流水线
```cpp
constexpr auto post_process_flags = 
    aiProcess_CalcTangentSpace | aiProcess_JoinIdenticalVertices |
    aiProcess_Triangulate | aiProcess_GenNormals | 
    aiProcess_SortByPType | aiProcess_FlipUVs;
```

## 常见模式

### 加载工作流
1. `load_scene()` → 创建 SceneLoader，用 Assimp 处理
2. `get_*_cnt()` → 查询对象数量
3. `get_*()` → 访问转换后的数据结构
4. `free_scene()` → 清理资源

### 数据访问模式
```cpp
const auto loader = load_scene(path);
for (size_t i = 0; i < get_instance_cnt(loader); ++i) {
    const auto instance = get_instance(loader, i);
    // 访问 instance->mesh_indices(), instance->world_transform 等
}
```

修改代码库时请注意：
- 在公共 API 中保持 C 兼容性
- 保持复杂结构体的仅移动语义
- 保持坐标系文档的准确性
- 同时测试 Visual Studio 和 Clang-cl 构建
- 验证 DLL 导出与依赖 DLL 正常工作
