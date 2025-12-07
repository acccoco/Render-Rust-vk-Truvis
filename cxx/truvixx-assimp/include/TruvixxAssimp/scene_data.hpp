#pragma once

#include "TruvixxAssimp/truvixx_assimp.export.h"

#include <cstdint>
#include <string>
#include <vector>

namespace truvixx
{

//=============================================================================
// 常量定义
//=============================================================================

inline constexpr size_t MAX_NAME_LENGTH = 256;

//=============================================================================
// SOA Mesh 数据 (Structure of Arrays)
//=============================================================================

/// Mesh 几何数据，采用 SOA 布局
/// 坐标系：右手系，X-Right，Y-Up，Z-Out
struct TRUVIXX_ASSIMP_API MeshData
{
    std::vector<float> positions;  ///< [x0,y0,z0, x1,y1,z1, ...] size = vertex_count * 3
    std::vector<float> normals;    ///< [nx0,ny0,nz0, ...] size = vertex_count * 3
    std::vector<float> tangents;   ///< [tx0,ty0,tz0, ...] size = vertex_count * 3
    std::vector<float> bitangents; ///< [bx0,by0,bz0, ...] size = vertex_count * 3
    std::vector<float> uvs;        ///< [u0,v0, u1,v1, ...] size = vertex_count * 2
    std::vector<uint32_t> indices; ///< 三角形索引 size = triangle_count * 3

    [[nodiscard]] uint32_t vertex_count() const noexcept
    {
        return static_cast<uint32_t>(positions.size() / 3);
    }

    [[nodiscard]] uint32_t index_count() const noexcept
    {
        return static_cast<uint32_t>(indices.size());
    }

    [[nodiscard]] uint32_t triangle_count() const noexcept
    {
        return static_cast<uint32_t>(indices.size() / 3);
    }

    /// 预分配内存
    void reserve(uint32_t vertex_count, uint32_t triangle_count);

    /// 清空数据
    void clear() noexcept;
};

//=============================================================================
// 材质数据
//=============================================================================

/// PBR 材质数据
struct TRUVIXX_ASSIMP_API MaterialData
{
    std::string name;

    // PBR 参数
    float base_color[4] = {1.0f, 1.0f, 1.0f, 1.0f}; ///< RGBA
    float roughness     = 0.5f;
    float metallic      = 0.0f;
    float emissive[4]   = {0.0f, 0.0f, 0.0f, 1.0f}; ///< RGBA
    float opacity       = 1.0f;                      ///< 1 = opaque, 0 = transparent

    // 纹理路径 (绝对路径)
    std::string diffuse_map;
    std::string normal_map;
};

//=============================================================================
// Instance 数据
//=============================================================================

/// 场景实例 (节点)
struct TRUVIXX_ASSIMP_API InstanceData
{
    std::string name;

    /// 世界变换矩阵 (列主序, 4x4)
    /// 坐标系：右手系，X-Right，Y-Up
    float world_transform[16] = {
        1, 0, 0, 0,  // col 0
        0, 1, 0, 0,  // col 1
        0, 0, 1, 0,  // col 2
        0, 0, 0, 1   // col 3
    };

    /// 该实例引用的 mesh 索引列表
    std::vector<uint32_t> mesh_indices;

    /// 该实例引用的材质索引列表 (与 mesh_indices 一一对应)
    std::vector<uint32_t> material_indices;

    [[nodiscard]] uint32_t mesh_count() const noexcept
    {
        return static_cast<uint32_t>(mesh_indices.size());
    }
};

//=============================================================================
// 完整场景数据
//=============================================================================

/// 场景容器，持有所有 mesh、材质和实例数据
struct TRUVIXX_ASSIMP_API SceneData
{
    std::vector<MeshData> meshes;
    std::vector<MaterialData> materials;
    std::vector<InstanceData> instances;

    /// 清空所有数据
    void clear() noexcept;

    [[nodiscard]] uint32_t mesh_count() const noexcept
    {
        return static_cast<uint32_t>(meshes.size());
    }

    [[nodiscard]] uint32_t material_count() const noexcept
    {
        return static_cast<uint32_t>(materials.size());
    }

    [[nodiscard]] uint32_t instance_count() const noexcept
    {
        return static_cast<uint32_t>(instances.size());
    }
};

} // namespace truvixx