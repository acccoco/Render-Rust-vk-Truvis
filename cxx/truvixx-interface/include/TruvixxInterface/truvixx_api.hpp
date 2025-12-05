#pragma once

/// @file truvixx_api.hpp
/// @brief Truvixx C FFI 接口
///
/// 本文件定义了 Truvixx 场景加载库的 C FFI 接口。
/// 设计原则:
/// - 所有结构体都是 POD (Plain Old Data)，可安全跨 FFI 边界传递
/// - 使用 "查询-分配-填充" 模式处理可变长度数据
/// - Mesh 数据采用 SOA (Structure of Arrays) 布局
/// - 返回 int (1=成功, 0=失败) 作为错误码
///
/// 坐标系约定:
/// - 右手坐标系: X-Right, Y-Up, Z-Out
/// - 矩阵存储: 列主序 (column-major)
/// - UV 原点: 左上角

#include "TruvixxInterface/truvixx_interface.export.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

//=============================================================================
// 句柄类型
//=============================================================================

/// 场景句柄 (不透明指针)
typedef struct TruvixxScene_* TruvixxScene;

//=============================================================================
// POD 结构体 (C 兼容, 可安全跨 FFI 传递)
//=============================================================================

/// 4x4 变换矩阵 (列主序)
/// m[0..3] = 第1列, m[4..7] = 第2列, ...
typedef struct TruvixxMat4
{
    float m[16];
} TruvixxMat4;

/// 材质信息
typedef struct TruvixxMaterial
{
    char name[256];           ///< 材质名称 (null 结尾)

    float base_color[4];      ///< RGBA 基础颜色
    float roughness;          ///< 粗糙度 [0, 1]
    float metallic;           ///< 金属度 [0, 1]
    float emissive[4];        ///< RGBA 自发光颜色
    float opacity;            ///< 不透明度: 1=opaque, 0=transparent

    char diffuse_map[256];    ///< 漫反射贴图路径 (null 结尾, 空字符串表示无)
    char normal_map[256];     ///< 法线贴图路径 (null 结尾, 空字符串表示无)
} TruvixxMaterial;

/// Instance 信息
typedef struct TruvixxInstance
{
    char name[256];              ///< 实例名称 (null 结尾)
    TruvixxMat4 world_transform; ///< 世界变换矩阵
    uint32_t mesh_count;         ///< 引用的 mesh 数量
    uint32_t _pad0;              ///< 对齐填充
} TruvixxInstance;

/// Mesh 元信息 (用于预分配 buffer)
typedef struct TruvixxMeshInfo
{
    uint32_t vertex_count;    ///< 顶点数量
    uint32_t index_count;     ///< 索引数量 (三角形数 * 3)
    uint32_t has_normals;     ///< 是否有法线 (1=有, 0=无)
    uint32_t has_tangents;    ///< 是否有切线 (1=有, 0=无)
    uint32_t has_uvs;         ///< 是否有 UV (1=有, 0=无)
    uint32_t _pad0;           ///< 对齐填充
} TruvixxMeshInfo;

//=============================================================================
// 场景生命周期
//=============================================================================

/// 加载场景文件
/// @param path 文件路径 (UTF-8)
/// @return 场景句柄, 失败返回 NULL
TRUVIXX_INTERFACE_API TruvixxScene truvixx_scene_load(const char* path);

/// 释放场景
/// @param scene 场景句柄 (可以为 NULL)
TRUVIXX_INTERFACE_API void truvixx_scene_free(TruvixxScene scene);

/// 获取加载错误信息
/// @param scene 场景句柄
/// @return 错误信息字符串, 无错误返回空字符串
TRUVIXX_INTERFACE_API const char* truvixx_scene_error(TruvixxScene scene);

//=============================================================================
// 场景查询
//=============================================================================

/// 获取 mesh 数量
TRUVIXX_INTERFACE_API uint32_t truvixx_scene_mesh_count(TruvixxScene scene);

/// 获取材质数量
TRUVIXX_INTERFACE_API uint32_t truvixx_scene_material_count(TruvixxScene scene);

/// 获取 instance 数量
TRUVIXX_INTERFACE_API uint32_t truvixx_scene_instance_count(TruvixxScene scene);

//=============================================================================
// 材质访问
//=============================================================================

/// 获取材质信息
/// @param scene 场景句柄
/// @param index 材质索引
/// @param out [out] 输出材质信息
/// @return 成功返回 1, 失败返回 0
TRUVIXX_INTERFACE_API int truvixx_material_get(
    TruvixxScene scene,
    uint32_t index,
    TruvixxMaterial* out
);

//=============================================================================
// Instance 访问
//=============================================================================

/// 获取 instance 信息
/// @param scene 场景句柄
/// @param index instance 索引
/// @param out [out] 输出 instance 信息
/// @return 成功返回 1, 失败返回 0
TRUVIXX_INTERFACE_API int truvixx_instance_get(
    TruvixxScene scene,
    uint32_t index,
    TruvixxInstance* out
);

/// 获取 instance 引用的 mesh 和材质索引
/// @param scene 场景句柄
/// @param index instance 索引
/// @param out_mesh_indices [out] mesh 索引数组 (大小 >= mesh_count), 可为 NULL
/// @param out_material_indices [out] 材质索引数组 (大小 >= mesh_count), 可为 NULL
/// @return 成功返回 1, 失败返回 0
TRUVIXX_INTERFACE_API int truvixx_instance_get_refs(
    TruvixxScene scene,
    uint32_t index,
    uint32_t* out_mesh_indices,
    uint32_t* out_material_indices
);

//=============================================================================
// Mesh 访问 (SOA 布局, 查询-分配-填充模式)
//=============================================================================

/// 获取 mesh 元信息 (用于预分配 buffer)
/// @param scene 场景句柄
/// @param index mesh 索引
/// @param out [out] 输出 mesh 元信息
/// @return 成功返回 1, 失败返回 0
TRUVIXX_INTERFACE_API int truvixx_mesh_get_info(
    TruvixxScene scene,
    uint32_t index,
    TruvixxMeshInfo* out
);

/// 填充顶点位置 (SOA: float[vertex_count * 3])
/// 布局: [x0, y0, z0, x1, y1, z1, ...]
TRUVIXX_INTERFACE_API int truvixx_mesh_fill_positions(
    TruvixxScene scene,
    uint32_t index,
    float* out
);

/// 填充顶点法线 (SOA: float[vertex_count * 3])
TRUVIXX_INTERFACE_API int truvixx_mesh_fill_normals(
    TruvixxScene scene,
    uint32_t index,
    float* out
);

/// 填充顶点切线 (SOA: float[vertex_count * 3])
TRUVIXX_INTERFACE_API int truvixx_mesh_fill_tangents(
    TruvixxScene scene,
    uint32_t index,
    float* out
);

/// 填充顶点 UV (SOA: float[vertex_count * 2])
/// 布局: [u0, v0, u1, v1, ...]
TRUVIXX_INTERFACE_API int truvixx_mesh_fill_uvs(
    TruvixxScene scene,
    uint32_t index,
    float* out
);

/// 填充索引 (uint32_t[index_count])
TRUVIXX_INTERFACE_API int truvixx_mesh_fill_indices(
    TruvixxScene scene,
    uint32_t index,
    uint32_t* out
);

/// 批量填充所有顶点属性
/// 任一指针为 NULL 则跳过该属性
/// @return 成功返回 1, 失败返回 0
TRUVIXX_INTERFACE_API int truvixx_mesh_fill_all(
    TruvixxScene scene,
    uint32_t index,
    float* out_positions,    ///< [vertex_count * 3] 或 NULL
    float* out_normals,      ///< [vertex_count * 3] 或 NULL
    float* out_tangents,     ///< [vertex_count * 3] 或 NULL
    float* out_uvs,          ///< [vertex_count * 2] 或 NULL
    uint32_t* out_indices    ///< [index_count] 或 NULL
);

#ifdef __cplusplus
}
#endif