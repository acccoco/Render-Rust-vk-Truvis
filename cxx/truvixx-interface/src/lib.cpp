#include "TruvixxInterface/lib.hpp"

#include "TruvixxAssimp/scene_loader.hpp"
#include "TruvixxAssimp/c_data_define.hpp"

#include <cstring>

//=============================================================================
// 内部辅助函数
//=============================================================================

namespace
{
/// 安全获取 SceneLoader 指针
inline truvis::SceneLoader* get_loader(void* handle)
{
    return static_cast<truvis::SceneLoader*>(handle);
}
} // namespace

//=============================================================================
// [DEPRECATED] 旧版接口实现 - 保留用于向后兼容
//=============================================================================

void* load_scene(const char* mesh_path)
{
    const auto loader = new truvis::SceneLoader(mesh_path);
    const auto load_ok = loader->load_scene();
    if (!load_ok)
    {
        free_scene(loader);
        return nullptr;
    }
    return loader;
}

void free_scene(void* loader)
{
    delete static_cast<truvis::SceneLoader*>(loader);
}

size_t get_mesh_cnt(void* loader)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_geometry_count() : 0;
}

size_t get_mat_cnt(void* loader)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_material_count() : 0;
}

size_t get_instance_cnt(void* loader)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_instance_count() : 0;
}

float* get_pos_buffer(void* loader, const size_t mesh_idx, size_t* vertex_cnt)
{
    const auto scene_loader = static_cast<truvis::SceneLoader*>(loader);
    return scene_loader->get_position(mesh_idx, *vertex_cnt);
}

const CxxInstance* get_instance(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_instance(idx) : nullptr;
}

const CxxRasterGeometry* get_mesh(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_geometry(idx) : nullptr;
}

const CxxMaterial* get_mat(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_material(idx) : nullptr;
}

//=============================================================================
// 新版接口实现 - C FFI 兼容，SOA 布局
//=============================================================================

//-----------------------------------------------------------------------------
// 场景生命周期
//-----------------------------------------------------------------------------

TruvixxSceneHandle truvixx_load_scene(const char* path)
{
    if (!path)
    {
        return nullptr;
    }

    auto* loader = new truvis::SceneLoader(path);
    if (!loader->load_scene())
    {
        delete loader;
        return nullptr;
    }
    return loader;
}

void truvixx_free_scene(TruvixxSceneHandle scene)
{
    delete get_loader(scene);
}

//-----------------------------------------------------------------------------
// 场景查询
//-----------------------------------------------------------------------------

uint32_t truvixx_get_mesh_count(TruvixxSceneHandle scene)
{
    return scene ? static_cast<uint32_t>(get_loader(scene)->get_geometry_count()) : 0;
}

uint32_t truvixx_get_material_count(TruvixxSceneHandle scene)
{
    return scene ? static_cast<uint32_t>(get_loader(scene)->get_material_count()) : 0;
}

uint32_t truvixx_get_instance_count(TruvixxSceneHandle scene)
{
    return scene ? static_cast<uint32_t>(get_loader(scene)->get_instance_count()) : 0;
}

//-----------------------------------------------------------------------------
// Material 访问
//-----------------------------------------------------------------------------

int truvixx_get_material(TruvixxSceneHandle scene, uint32_t index, TruvixxMaterial* out_material)
{
    if (!scene || !out_material)
    {
        return 0;
    }

    const auto* mat = get_loader(scene)->get_material(index);
    if (!mat)
    {
        return 0;
    }

    // 复制材质数据到输出结构
    std::memcpy(out_material->name, mat->name, sizeof(out_material->name));

    out_material->base_color[0] = mat->base_color.x;
    out_material->base_color[1] = mat->base_color.y;
    out_material->base_color[2] = mat->base_color.z;
    out_material->base_color[3] = mat->base_color.w;

    out_material->roughness_factor = mat->roughness_factor;
    out_material->metallic_factor = mat->metallic_factor;

    out_material->emissive_color[0] = mat->emissive_color.x;
    out_material->emissive_color[1] = mat->emissive_color.y;
    out_material->emissive_color[2] = mat->emissive_color.z;
    out_material->emissive_color[3] = mat->emissive_color.w;

    out_material->opacity = mat->opaque_factor;

    std::memcpy(out_material->diffuse_map, mat->diffuse_map, sizeof(out_material->diffuse_map));
    std::memcpy(out_material->normal_map, mat->normal_map, sizeof(out_material->normal_map));

    return 1;
}

//-----------------------------------------------------------------------------
// Instance 访问
//-----------------------------------------------------------------------------

int truvixx_get_instance_info(TruvixxSceneHandle scene, uint32_t index, TruvixxInstanceInfo* out_info)
{
    if (!scene || !out_info)
    {
        return 0;
    }

    const auto* inst = get_loader(scene)->get_instance(index);
    if (!inst)
    {
        return 0;
    }

    std::memcpy(out_info->name, inst->name, sizeof(out_info->name));
    std::memcpy(out_info->world_transform.m, inst->world_transform.m, sizeof(out_info->world_transform.m));
    out_info->mesh_count = inst->mesh_cnt();
    out_info->_padding = 0;

    return 1;
}

int truvixx_get_instance_mesh_refs(
    TruvixxSceneHandle scene,
    uint32_t instance_index,
    uint32_t* out_mesh_indices,
    uint32_t* out_material_indices)
{
    if (!scene)
    {
        return 0;
    }

    const auto* inst = get_loader(scene)->get_instance(instance_index);
    if (!inst)
    {
        return 0;
    }

    const uint32_t count = inst->mesh_cnt();

    if (out_mesh_indices)
    {
        for (uint32_t i = 0; i < count; ++i)
        {
            out_mesh_indices[i] = inst->mesh_indices()[i];
        }
    }

    if (out_material_indices)
    {
        for (uint32_t i = 0; i < count; ++i)
        {
            out_material_indices[i] = inst->mat_indices()[i];
        }
    }

    return 1;
}

//-----------------------------------------------------------------------------
// Mesh 数据访问 (SOA 布局)
//-----------------------------------------------------------------------------

int truvixx_get_mesh_info(TruvixxSceneHandle scene, uint32_t mesh_index, TruvixxMeshInfo* out_info)
{
    if (!scene || !out_info)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    out_info->vertex_count = geo->vertex_cnt();
    out_info->index_count = geo->face_cnt() * 3; // 每个三角形面 3 个索引

    return 1;
}

int truvixx_fill_mesh_positions(TruvixxSceneHandle scene, uint32_t mesh_index, float* out_positions)
{
    if (!scene || !out_positions)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    const auto* vertices = geo->vertices();
    const uint32_t count = geo->vertex_cnt();

    for (uint32_t i = 0; i < count; ++i)
    {
        out_positions[i * 3 + 0] = vertices[i].position.x;
        out_positions[i * 3 + 1] = vertices[i].position.y;
        out_positions[i * 3 + 2] = vertices[i].position.z;
    }

    return 1;
}

int truvixx_fill_mesh_normals(TruvixxSceneHandle scene, uint32_t mesh_index, float* out_normals)
{
    if (!scene || !out_normals)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    const auto* vertices = geo->vertices();
    const uint32_t count = geo->vertex_cnt();

    for (uint32_t i = 0; i < count; ++i)
    {
        out_normals[i * 3 + 0] = vertices[i].normal.x;
        out_normals[i * 3 + 1] = vertices[i].normal.y;
        out_normals[i * 3 + 2] = vertices[i].normal.z;
    }

    return 1;
}

int truvixx_fill_mesh_tangents(TruvixxSceneHandle scene, uint32_t mesh_index, float* out_tangents)
{
    if (!scene || !out_tangents)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    const auto* vertices = geo->vertices();
    const uint32_t count = geo->vertex_cnt();

    for (uint32_t i = 0; i < count; ++i)
    {
        out_tangents[i * 3 + 0] = vertices[i].tangent.x;
        out_tangents[i * 3 + 1] = vertices[i].tangent.y;
        out_tangents[i * 3 + 2] = vertices[i].tangent.z;
    }

    return 1;
}

int truvixx_fill_mesh_uvs(TruvixxSceneHandle scene, uint32_t mesh_index, float* out_uvs)
{
    if (!scene || !out_uvs)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    const auto* vertices = geo->vertices();
    const uint32_t count = geo->vertex_cnt();

    for (uint32_t i = 0; i < count; ++i)
    {
        out_uvs[i * 2 + 0] = vertices[i].uv.x;
        out_uvs[i * 2 + 1] = vertices[i].uv.y;
    }

    return 1;
}

int truvixx_fill_mesh_indices(TruvixxSceneHandle scene, uint32_t mesh_index, uint32_t* out_indices)
{
    if (!scene || !out_indices)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    const auto* faces = geo->faces();
    const uint32_t face_count = geo->face_cnt();

    for (uint32_t i = 0; i < face_count; ++i)
    {
        out_indices[i * 3 + 0] = faces[i].a;
        out_indices[i * 3 + 1] = faces[i].b;
        out_indices[i * 3 + 2] = faces[i].c;
    }

    return 1;
}

int truvixx_fill_mesh_all(
    TruvixxSceneHandle scene,
    uint32_t mesh_index,
    float* out_positions,
    float* out_normals,
    float* out_tangents,
    float* out_uvs,
    uint32_t* out_indices)
{
    if (!scene)
    {
        return 0;
    }

    const auto* geo = get_loader(scene)->get_geometry(mesh_index);
    if (!geo)
    {
        return 0;
    }

    const auto* vertices = geo->vertices();
    const uint32_t vertex_count = geo->vertex_cnt();

    // 填充顶点属性 (SOA 布局)
    for (uint32_t i = 0; i < vertex_count; ++i)
    {
        if (out_positions)
        {
            out_positions[i * 3 + 0] = vertices[i].position.x;
            out_positions[i * 3 + 1] = vertices[i].position.y;
            out_positions[i * 3 + 2] = vertices[i].position.z;
        }
        if (out_normals)
        {
            out_normals[i * 3 + 0] = vertices[i].normal.x;
            out_normals[i * 3 + 1] = vertices[i].normal.y;
            out_normals[i * 3 + 2] = vertices[i].normal.z;
        }
        if (out_tangents)
        {
            out_tangents[i * 3 + 0] = vertices[i].tangent.x;
            out_tangents[i * 3 + 1] = vertices[i].tangent.y;
            out_tangents[i * 3 + 2] = vertices[i].tangent.z;
        }
        if (out_uvs)
        {
            out_uvs[i * 2 + 0] = vertices[i].uv.x;
            out_uvs[i * 2 + 1] = vertices[i].uv.y;
        }
    }

    // 填充索引
    if (out_indices)
    {
        const auto* faces = geo->faces();
        const uint32_t face_count = geo->face_cnt();

        for (uint32_t i = 0; i < face_count; ++i)
        {
            out_indices[i * 3 + 0] = faces[i].a;
            out_indices[i * 3 + 1] = faces[i].b;
            out_indices[i * 3 + 2] = faces[i].c;
        }
    }

    return 1;
}