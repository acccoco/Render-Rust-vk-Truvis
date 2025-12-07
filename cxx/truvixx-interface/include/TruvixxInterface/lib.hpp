#pragma once
#include "truvixx_interface.export.h"
#include "TruvixxAssimp/c_data_define.hpp"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

//=============================================================================
// [DEPRECATED] 旧版接口 - 保留用于向后兼容，请使用新版 truvixx_* 接口
//=============================================================================

#pragma region deprecated_assimp
/// @deprecated 使用 truvixx_load_scene() 替代
TRUVIXX_INTERFACE_API void* load_scene(const char* mesh_path);

/// @deprecated 使用 truvixx_free_scene() 替代
TRUVIXX_INTERFACE_API void free_scene(void* loader);

/// @deprecated 使用 truvixx_get_mesh_count() 替代
TRUVIXX_INTERFACE_API size_t get_mesh_cnt(void* loader);

/// @deprecated 使用 truvixx_get_material_count() 替代
TRUVIXX_INTERFACE_API size_t get_mat_cnt(void* loader);

/// @deprecated 使用 truvixx_get_instance_count() 替代
TRUVIXX_INTERFACE_API size_t get_instance_cnt(void* loader);

/// @deprecated 使用 truvixx_fill_mesh_positions() 替代
TRUVIXX_INTERFACE_API float* get_pos_buffer(void* loader, size_t mesh_idx, size_t* vertex_cnt);

/// @deprecated 使用 truvixx_get_instance_info() + truvixx_get_instance_mesh_refs() 替代
TRUVIXX_INTERFACE_API const CxxInstance* get_instance(void* loader, size_t idx);

/// @deprecated 使用 truvixx_get_mesh_info() + truvixx_fill_mesh_* 系列函数替代
TRUVIXX_INTERFACE_API const CxxRasterGeometry* get_mesh(void* loader, size_t idx);

/// @deprecated 使用 truvixx_get_material() 替代
TRUVIXX_INTERFACE_API const CxxMaterial* get_mat(void* loader, size_t idx);
#pragma endregion

//=============================================================================
// 新版接口 - C FFI 兼容，SOA 布局，查询-分配-填充模式
//=============================================================================

//-----------------------------------------------------------------------------
// 句柄类型 (不透明指针)
//-----------------------------------------------------------------------------

/// 场景句柄，由 truvixx_load_scene() 返回
typedef void* TruvixxSceneHandle;

//-----------------------------------------------------------------------------
// POD 结构体定义 (alignas(4), C 兼容)
//-----------------------------------------------------------------------------

/// 4x4 矩阵 (列主序)
/// 坐标系：右手系，X-Right，Y-Up
typedef struct TruvixxMat4
{
    float m[16]; ///< m[0..3] 是第一列，以此类推
} TruvixxMat4;

/// 材质信息 (POD, C 兼容)
typedef struct TruvixxMaterial
{
    char name[256]; ///< 材质名称，null 结尾

    float base_color[4];     ///< RGBA 基础颜色
    float roughness_factor;  ///< 粗糙度因子
    float metallic_factor;   ///< 金属度因子
    float emissive_color[4]; ///< RGBA 自发光颜色
    float opacity;           ///< 不透明度：1 = opaque, 0 = transparent

    char diffuse_map[256]; ///< 漫反射贴图路径，null 结尾
    char normal_map[256];  ///< 法线贴图路径，null 结尾
} TruvixxMaterial;

/// Instance 基础信息 (POD, 不含动态数组)
typedef struct TruvixxInstanceInfo
{
    char name[256];              ///< 实例名称，null 结尾
    TruvixxMat4 world_transform; ///< 世界变换矩阵 (列主序)
    uint32_t mesh_count;         ///< 该 instance 引用的 mesh 数量
    uint32_t _padding;           ///< 对齐填充
} TruvixxInstanceInfo;

/// Mesh 元信息 (用于查询分配空间)
typedef struct TruvixxMeshInfo
{
    uint32_t vertex_count; ///< 顶点数量
    uint32_t index_count;  ///< 索引数量 (三角形面数 * 3)
} TruvixxMeshInfo;

//-----------------------------------------------------------------------------
// 场景生命周期
//-----------------------------------------------------------------------------

/// 加载场景文件
/// @param path 场景文件路径 (UTF-8 编码)
/// @return 场景句柄，失败返回 NULL
TRUVIXX_INTERFACE_API TruvixxSceneHandle truvixx_load_scene(const char* path);

/// 释放场景资源
/// @param scene 场景句柄，必须是 truvixx_load_scene() 返回的值
TRUVIXX_INTERFACE_API void truvixx_free_scene(TruvixxSceneHandle scene);

//-----------------------------------------------------------------------------
// 场景查询
//-----------------------------------------------------------------------------

/// 获取场景中的 mesh 数量
TRUVIXX_INTERFACE_API uint32_t truvixx_get_mesh_count(TruvixxSceneHandle scene);

/// 获取场景中的材质数量
TRUVIXX_INTERFACE_API uint32_t truvixx_get_material_count(TruvixxSceneHandle scene);

/// 获取场景中的 instance 数量
TRUVIXX_INTERFACE_API uint32_t truvixx_get_instance_count(TruvixxSceneHandle scene);

//-----------------------------------------------------------------------------
// Material 访问
//-----------------------------------------------------------------------------

/// 获取材质信息
/// @param scene 场景句柄
/// @param index 材质索引
/// @param out_material [out] 输出材质信息
/// @return 成功返回 1，失败返回 0
TRUVIXX_INTERFACE_API int truvixx_get_material(
    TruvixxSceneHandle scene,
    uint32_t index,
    TruvixxMaterial* out_material
);

//-----------------------------------------------------------------------------
// Instance 访问
//-----------------------------------------------------------------------------

/// 获取 Instance 基础信息
/// @param scene 场景句柄
/// @param index Instance 索引
/// @param out_info [out] 输出 Instance 信息
/// @return 成功返回 1，失败返回 0
TRUVIXX_INTERFACE_API int truvixx_get_instance_info(
    TruvixxSceneHandle scene,
    uint32_t index,
    TruvixxInstanceInfo* out_info
);

/// 获取 Instance 引用的 mesh 和材质索引列表
/// @param scene 场景句柄
/// @param instance_index Instance 索引
/// @param out_mesh_indices [out] 外部分配的数组，大小 >= mesh_count
/// @param out_material_indices [out] 外部分配的数组，大小 >= mesh_count
/// @return 成功返回 1，失败返回 0
TRUVIXX_INTERFACE_API int truvixx_get_instance_mesh_refs(
    TruvixxSceneHandle scene,
    uint32_t instance_index,
    uint32_t* out_mesh_indices,
    uint32_t* out_material_indices
);

//-----------------------------------------------------------------------------
// Mesh 数据访问 (SOA 布局, 查询-分配-填充模式)
//-----------------------------------------------------------------------------

/// 查询 Mesh 元信息 (用于外部预分配 buffer)
/// @param scene 场景句柄
/// @param mesh_index Mesh 索引
/// @param out_info [out] 输出 Mesh 元信息
/// @return 成功返回 1，失败返回 0
TRUVIXX_INTERFACE_API int truvixx_get_mesh_info(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    TruvixxMeshInfo* out_info
);

/// 填充顶点位置数据 (SOA 布局)
/// @param scene 场景句柄
/// @param mesh_index Mesh 索引
/// @param out_positions [out] 外部分配的 buffer，大小 >= vertex_count * 3 * sizeof(float)
///        布局: [x0, y0, z0, x1, y1, z1, ...]
/// @return 成功返回 1，失败返回 0
TRUVIXX_INTERFACE_API int truvixx_fill_mesh_positions(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    float* out_positions
);

/// 填充顶点法线数据 (SOA 布局)
/// @param out_normals [out] 外部分配的 buffer，大小 >= vertex_count * 3 * sizeof(float)
TRUVIXX_INTERFACE_API int truvixx_fill_mesh_normals(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    float* out_normals
);

/// 填充顶点切线数据 (SOA 布局)
/// @param out_tangents [out] 外部分配的 buffer，大小 >= vertex_count * 3 * sizeof(float)
TRUVIXX_INTERFACE_API int truvixx_fill_mesh_tangents(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    float* out_tangents
);

/// 填充顶点 UV 数据 (SOA 布局)
/// @param out_uvs [out] 外部分配的 buffer，大小 >= vertex_count * 2 * sizeof(float)
///        布局: [u0, v0, u1, v1, ...]
TRUVIXX_INTERFACE_API int truvixx_fill_mesh_uvs(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    float* out_uvs
);

/// 填充索引数据
/// @param out_indices [out] 外部分配的 buffer，大小 >= index_count * sizeof(uint32_t)
TRUVIXX_INTERFACE_API int truvixx_fill_mesh_indices(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    uint32_t* out_indices
);

/// 批量填充所有顶点属性 (减少多次调用开销)
/// @param out_positions [out] [vertex_count * 3] 或 NULL 跳过
/// @param out_normals [out] [vertex_count * 3] 或 NULL 跳过
/// @param out_tangents [out] [vertex_count * 3] 或 NULL 跳过
/// @param out_uvs [out] [vertex_count * 2] 或 NULL 跳过
/// @param out_indices [out] [index_count] 或 NULL 跳过
/// @return 成功返回 1，失败返回 0
TRUVIXX_INTERFACE_API int truvixx_fill_mesh_all(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    float* out_positions,
    float* out_normals,
    float* out_tangents,
    float* out_uvs,
    uint32_t* out_indices
);

#ifdef __cplusplus
}
#endif