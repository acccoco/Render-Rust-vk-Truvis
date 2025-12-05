#include "TruvixxAssimp/scene_data.hpp"

namespace truvixx
{

void MeshData::reserve(uint32_t vertex_count, uint32_t triangle_count)
{
    positions.reserve(static_cast<size_t>(vertex_count) * 3);
    normals.reserve(static_cast<size_t>(vertex_count) * 3);
    tangents.reserve(static_cast<size_t>(vertex_count) * 3);
    bitangents.reserve(static_cast<size_t>(vertex_count) * 3);
    uvs.reserve(static_cast<size_t>(vertex_count) * 2);
    indices.reserve(static_cast<size_t>(triangle_count) * 3);
}

void MeshData::clear() noexcept
{
    positions.clear();
    normals.clear();
    tangents.clear();
    bitangents.clear();
    uvs.clear();
    indices.clear();
}

void SceneData::clear() noexcept
{
    meshes.clear();
    materials.clear();
    instances.clear();
}

} // namespace truvixx