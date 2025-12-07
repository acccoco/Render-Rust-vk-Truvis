# Truvixx C++ 场景加载器 - AI 编程指南

## 项目概述
Truvixx 是为 Rust 渲染引擎 Truvis 提供 3D 资产加载能力的 C++ 库。项目生成两个 DLL：
- `truvixx-assimp.dll`：基于 Assimp 的场景加载核心
- `truvixx-interface.dll`：对外暴露 C FFI 接口，供 Rust 调用

## 架构设计

```
truvixx-interface (C FFI 层)
    └── truvixx-assimp (场景加载核心)
            └── Assimp + GLM (第三方依赖)
```

### 模块职责
| 模块 | 目录 | 职责 |
|------|------|------|
| `truvixx-assimp` | `truvixx-assimp/` | Assimp 封装、数据结构定义、坐标转换 |
| `truvixx-interface` | `truvixx-interface/` | C FFI 导出函数、void* 不透明指针封装 |

### 关键文件
- `TruvixxInterface/lib.hpp`：FFI 函数声明，`extern "C"` 接口
- `TruvixxAssimp/c_data_define.hpp`：C 兼容的 POD 结构体（`alignas(4)`）
- `TruvixxAssimp/scene_loader.hpp`：Assimp 场景处理逻辑

## 构建命令

使用 CMake Presets（推荐）：
```powershell
# Visual Studio 2022
cmake --preset vs2022
cmake --build --preset debug   # 或 --preset release

# Clang-cl + Ninja
cmake --preset clang-cl-debug
cmake --build --preset clang-debug
```

输出位置：`build/Debug/` 或 `build/Release/`

## 依赖管理
项目使用 vcpkg manifest 模式（`vcpkg.json`），**不要**使用 `vcpkg install` 命令。依赖版本已锁定：
- Assimp 5.4.3
- GLM 1.0.1#3

## 编码约定

### C FFI 接口规范
```cpp
// ✅ 正确：void* + extern "C"，空指针检查
extern "C" TRUVIXX_INTERFACE_API void* load_scene(const char* path);
extern "C" TRUVIXX_INTERFACE_API size_t get_mesh_cnt(void* loader);  // loader 为空时返回 0

// ❌ 错误：不要在 FFI 层抛异常或返回 C++ 对象
```

### 数据结构设计
```cpp
// C 兼容结构体必须使用 alignas(4)
struct alignas(4) CxxVec3f { float x, y, z; };

// 字符串使用固定大小数组，而非 std::string
char name[PATH_BUFFER_SIZE];  // PATH_BUFFER_SIZE = 256

// 复杂类型采用仅移动语义，禁止拷贝
CxxRasterGeometry(CxxRasterGeometry&&) noexcept;           // ✅ 允许移动
CxxRasterGeometry(const CxxRasterGeometry&) = delete;      // ❌ 禁止拷贝
```

### 坐标系约定
- **右手坐标系**：X-Right, Y-Up, Z-Out
- **UV 原点**：左上角（通过 `aiProcess_FlipUVs` 实现）
- **矩阵存储**：列主序（`CxxMat4f.m[0..3]` 是第一列）

## DLL 导出宏
CMake 自动生成导出头文件，使用 `TRUVIXX_*_API` 宏：
```cpp
#include "TruvixxAssimp/truvixx_assimp.export.h"
struct TRUVIXX_ASSIMP_API CxxMaterial { ... };
```

## API 使用模式
```cpp
void* loader = load_scene("scene.gltf");
if (!loader) { /* 加载失败 */ }

for (size_t i = 0; i < get_instance_cnt(loader); ++i) {
    const CxxInstance* inst = get_instance(loader, i);
    // inst->mesh_indices(), inst->world_transform, inst->name
}

free_scene(loader);  // 必须手动释放
```

## 修改检查清单
- [ ] FFI 函数保持 `extern "C"` 和 void* 签名
- [ ] 新增结构体使用 `alignas(4)` 和固定大小数组
- [ ] 复杂类型实现移动语义，删除拷贝构造/赋值
- [ ] 坐标系注释保持准确："坐标系：右手系，X-Right，Y-Up"
- [ ] 同时测试 VS2022 和 Clang-cl 构建
