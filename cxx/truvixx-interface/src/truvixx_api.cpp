#include "TruvixxInterface/truvixx_api.hpp"
#include "TruvixxAssimp/scene_importer.hpp"

#include <algorithm>
#include <cstring>

//=============================================================================
// 内部类型定义
//=============================================================================

/// 场景句柄的实际类型
struct TruvixxScene_
{
    truvixx::SceneImporter importer;
};

//=============================================================================
// 辅助函数
//=============================================================================

namespace
{

/// 安全复制字符串到固定大小缓冲区
inline void safe_strcpy(char* dest, size_t dest_size, const std::string& src)
{
    if (dest_size == 0)
        return;

    size_t copy_len = std::min(src.size(), dest_size - 1);
    std::memcpy(dest, src.data(), copy_len);
    dest[copy_len] = '\0';
}

/// 获取场景数据 (带空检查)
inline const truvixx::SceneData* get_scene_data(TruvixxScene scene)
{
    if (!scene || !scene->importer.is_loaded())
        return nullptr;
    return &scene->importer.scene();
}

} // namespace

//=============================================================================
// 场景生命周期
//=============================================================================

TruvixxScene truvixx_scene_load(const char* path)
{
    if (!path)
        return nullptr;

    auto* scene = new TruvixxScene_;
    if (!scene->importer.load(path))
    {
        // 保留错误信息，不立即删除
        return scene;
    }
    return scene;
}

void truvixx_scene_free(TruvixxScene scene)
{
    delete scene;
}

const char* truvixx_scene_error(TruvixxScene scene)
{
    if (!scene)
        return "";
    return scene->importer.error().c_str();
}

//=============================================================================
// 场景查询
//=============================================================================

uint32_t truvixx_scene_mesh_count(TruvixxScene scene)
{
    const auto* data = get_scene_data(scene);
    return data ? data->mesh_count() : 0;
}

uint32_t truvixx_scene_material_count(TruvixxScene scene)
{
    const auto* data = get_scene_data(scene);
    return data ? data->material_count() : 0;
}

uint32_t truvixx_scene_instance_count(TruvixxScene scene)
{
    const auto* data = get_scene_data(scene);
    return data ? data->instance_count() : 0;
}

//=============================================================================
// 材质访问
//=============================================================================

int truvixx_material_get(TruvixxScene scene, uint32_t index, TruvixxMaterial* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->material_count())
        return 0;

    const auto& mat = data->materials[index];

    safe_strcpy(out->name, sizeof(out->name), mat.name);

    std::memcpy(out->base_color, mat.base_color, sizeof(out->base_color));
    out->roughness = mat.roughness;
    out->metallic  = mat.metallic;
    std::memcpy(out->emissive, mat.emissive, sizeof(out->emissive));
    out->opacity = mat.opacity;

    safe_strcpy(out->diffuse_map, sizeof(out->diffuse_map), mat.diffuse_map);
    safe_strcpy(out->normal_map, sizeof(out->normal_map), mat.normal_map);

    return 1;
}

//=============================================================================
// Instance 访问
//=============================================================================

int truvixx_instance_get(TruvixxScene scene, uint32_t index, TruvixxInstance* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->instance_count())
        return 0;

    const auto& inst = data->instances[index];

    safe_strcpy(out->name, sizeof(out->name), inst.name);
    std::memcpy(out->world_transform.m, inst.world_transform, sizeof(out->world_transform.m));
    out->mesh_count = inst.mesh_count();
    out->_pad0 = 0;

    return 1;
}

int truvixx_instance_get_refs(
    TruvixxScene scene,
    uint32_t index,
    uint32_t* out_mesh_indices,
    uint32_t* out_material_indices)
{
    const auto* data = get_scene_data(scene);
    if (!data || index >= data->instance_count())
        return 0;

    const auto& inst = data->instances[index];

    if (out_mesh_indices)
    {
        std::memcpy(out_mesh_indices, inst.mesh_indices.data(),
                    inst.mesh_indices.size() * sizeof(uint32_t));
    }

    if (out_material_indices)
    {
        std::memcpy(out_material_indices, inst.material_indices.data(),
                    inst.material_indices.size() * sizeof(uint32_t));
    }

    return 1;
}

//=============================================================================
// Mesh 访问
//=============================================================================

int truvixx_mesh_get_info(TruvixxScene scene, uint32_t index, TruvixxMeshInfo* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& mesh = data->meshes[index];

    out->vertex_count = mesh.vertex_count();
    out->index_count  = mesh.index_count();
    out->has_normals  = mesh.normals.empty() ? 0 : 1;
    out->has_tangents = mesh.tangents.empty() ? 0 : 1;
    out->has_uvs      = mesh.uvs.empty() ? 0 : 1;
    out->_pad0        = 0;

    return 1;
}

int truvixx_mesh_fill_positions(TruvixxScene scene, uint32_t index, float* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& positions = data->meshes[index].positions;
    std::memcpy(out, positions.data(), positions.size() * sizeof(float));

    return 1;
}

int truvixx_mesh_fill_normals(TruvixxScene scene, uint32_t index, float* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& normals = data->meshes[index].normals;
    if (normals.empty())
        return 0;

    std::memcpy(out, normals.data(), normals.size() * sizeof(float));

    return 1;
}

int truvixx_mesh_fill_tangents(TruvixxScene scene, uint32_t index, float* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& tangents = data->meshes[index].tangents;
    if (tangents.empty())
        return 0;

    std::memcpy(out, tangents.data(), tangents.size() * sizeof(float));

    return 1;
}

int truvixx_mesh_fill_uvs(TruvixxScene scene, uint32_t index, float* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& uvs = data->meshes[index].uvs;
    if (uvs.empty())
        return 0;

    std::memcpy(out, uvs.data(), uvs.size() * sizeof(float));

    return 1;
}

int truvixx_mesh_fill_indices(TruvixxScene scene, uint32_t index, uint32_t* out)
{
    if (!out)
        return 0;

    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& indices = data->meshes[index].indices;
    std::memcpy(out, indices.data(), indices.size() * sizeof(uint32_t));

    return 1;
}

int truvixx_mesh_fill_all(
    TruvixxScene scene,
    uint32_t index,
    float* out_positions,
    float* out_normals,
    float* out_tangents,
    float* out_uvs,
    uint32_t* out_indices)
{
    const auto* data = get_scene_data(scene);
    if (!data || index >= data->mesh_count())
        return 0;

    const auto& mesh = data->meshes[index];

    if (out_positions && !mesh.positions.empty())
    {
        std::memcpy(out_positions, mesh.positions.data(),
                    mesh.positions.size() * sizeof(float));
    }

    if (out_normals && !mesh.normals.empty())
    {
        std::memcpy(out_normals, mesh.normals.data(),
                    mesh.normals.size() * sizeof(float));
    }

    if (out_tangents && !mesh.tangents.empty())
    {
        std::memcpy(out_tangents, mesh.tangents.data(),
                    mesh.tangents.size() * sizeof(float));
    }

    if (out_uvs && !mesh.uvs.empty())
    {
        std::memcpy(out_uvs, mesh.uvs.data(),
                    mesh.uvs.size() * sizeof(float));
    }

    if (out_indices && !mesh.indices.empty())
    {
        std::memcpy(out_indices, mesh.indices.data(),
                    mesh.indices.size() * sizeof(uint32_t));
    }

    return 1;
}