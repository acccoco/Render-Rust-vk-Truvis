#pragma once

#include "TruvixxAssimp/base_type.h"
#include "TruvixxInterface/truvixx_interface.export.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef unsigned int uint32_t;
typedef int int32_t;

typedef enum : uint32_t
{
    ResTypeFail = 0,
    ResTypeSuccess = 1,
} ResType;

/// 场景句柄 (不透明指针)
typedef struct TruvixxScene* TruvixxSceneHandle;

/// 材质信息
typedef struct
{
    char name[256];

    TruvixxFloat4 base_color;
    float roughness;
    TruvixxFloat4 emissive;
    float metallic;
    float opacity;

    char diffuse_map[256];
    char normal_map[256];
} TruvixxMat;

/// Instance 信息
typedef struct
{
    char name[256];
    TruvixxFloat4x4 world_transform; ///< 世界变换矩阵
    unsigned int mesh_count;
} TruvixxInstance;

/// Mesh 元信息 (用于预分配 buffer)
typedef struct
{
    uint32_t vertex_count;
    uint32_t index_count;

    uint32_t has_normals;
    uint32_t has_tangents;
    uint32_t has_uvs;
} TruvixxMeshInfo;

#pragma region 场景生命周期

/// 加载场景文件
/// @param path 文件路径 (UTF-8)
/// @return 场景句柄, 失败返回 NULL
TruvixxSceneHandle TRUVIXX_INTERFACE_API truvixx_scene_load(const char* path);

/// 释放场景
/// @param scene 场景句柄 (可以为 NULL)
void TRUVIXX_INTERFACE_API truvixx_scene_free(TruvixxSceneHandle scene);

#pragma endregion

#pragma region Scene

/// 获取 mesh 数量
uint32_t TRUVIXX_INTERFACE_API truvixx_scene_mesh_count(TruvixxSceneHandle scene);

/// 获取材质数量
uint32_t TRUVIXX_INTERFACE_API truvixx_scene_material_count(TruvixxSceneHandle scene);

/// 获取 instance 数量
uint32_t TRUVIXX_INTERFACE_API truvixx_scene_instance_count(TruvixxSceneHandle scene);

#pragma endregion

#pragma region Instance访问

/// 获取 instance 信息
/// @param scene 场景句柄
/// @param index instance 索引
/// @param out [out] 输出 instance 信息
/// @return 成功返回 1, 失败返回 0
ResType TRUVIXX_INTERFACE_API truvixx_instance_get(TruvixxSceneHandle scene, uint32_t index, TruvixxInstance* out);

/// 获取 instance 引用的 mesh 和材质索引
/// @param scene 场景句柄
/// @param instance_index instance 索引
/// @param out_mesh_indices [out] mesh 索引数组 (大小 >= mesh_count), 可为 NULL
/// @param out_material_indices [out] 材质索引数组 (大小 >= mesh_count), 可为 NULL
/// @return 成功返回 1, 失败返回 0
ResType TRUVIXX_INTERFACE_API truvixx_instance_get_refs(
    TruvixxSceneHandle scene,
    uint32_t instance_index,
    uint32_t* out_mesh_indices,
    uint32_t* out_material_indices
);

#pragma endregion

#pragma region 材质访问

ResType TRUVIXX_INTERFACE_API truvixx_material_get(TruvixxSceneHandle scene, uint32_t mat_index, TruvixxMat* out);

#pragma endregion

#pragma region Mesh访问
// SOA 布局, 查询-分配-填充模式

/// 获取 mesh 元信息 (用于预分配 buffer)
/// @param scene 场景句柄
/// @param mesh_index mesh 索引
/// @param out [out] 输出 mesh 元信息
/// @return 成功返回 1, 失败返回 0
TRUVIXX_INTERFACE_API ResType truvixx_mesh_get_info(TruvixxSceneHandle scene, uint32_t mesh_index, TruvixxMeshInfo* out);

TRUVIXX_INTERFACE_API ResType truvixx_mesh_fill_positions(TruvixxSceneHandle scene, uint32_t mesh_index, float* out);
TRUVIXX_INTERFACE_API ResType truvixx_mesh_fill_normals(TruvixxSceneHandle scene, uint32_t mesh_index, float* out);
TRUVIXX_INTERFACE_API ResType truvixx_mesh_fill_tangents(TruvixxSceneHandle scene, uint32_t mesh_index, float* out);
TRUVIXX_INTERFACE_API ResType truvixx_mesh_fill_uvs(TruvixxSceneHandle scene, uint32_t mesh_index, float* out);
TRUVIXX_INTERFACE_API ResType truvixx_mesh_fill_indices(TruvixxSceneHandle scene, uint32_t mesh_index, uint32_t* out);

TRUVIXX_INTERFACE_API const TruvixxFloat3* truvixx_mesh_get_positions(TruvixxSceneHandle scene, uint32_t mesh_index);
TRUVIXX_INTERFACE_API const TruvixxFloat3* truvixx_mesh_get_normals(TruvixxSceneHandle scene, uint32_t mesh_index);
TRUVIXX_INTERFACE_API const TruvixxFloat3* truvixx_mesh_get_tangents(TruvixxSceneHandle scene, uint32_t mesh_index);
TRUVIXX_INTERFACE_API const TruvixxFloat2* truvixx_mesh_get_uvs(TruvixxSceneHandle scene, uint32_t mesh_index);
TRUVIXX_INTERFACE_API const uint32_t* truvixx_mesh_get_indices(TruvixxSceneHandle scene, uint32_t mesh_index);

#pragma endregion

#ifdef __cplusplus
}
#endif
