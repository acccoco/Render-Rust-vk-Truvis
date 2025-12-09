#include "TruvixxAssimp/scene_importer.hpp"

#include <assimp/Importer.hpp>
#include <assimp/postprocess.h>
#include <assimp/scene.h>
#include <assimp/matrix4x4.h>
#include <deque>
#include <format>

namespace truvixx
{

//=============================================================================
// 构造/析构
//=============================================================================

SceneImporter::SceneImporter()
    : importer_(std::make_unique<Assimp::Importer>())
{}

SceneImporter::~SceneImporter() = default;

//=============================================================================
// 公共接口
//=============================================================================

bool SceneImporter::load(const std::filesystem::path& path)
{
    // 清理之前的状态
    clear();

    // 验证文件存在
    if (!std::filesystem::exists(path) || !std::filesystem::is_regular_file(path))
    {
        error_msg_ = std::format("File not found: {}", path.string());
        return false;
    }

    dir_ = path.parent_path();

    // Assimp 后处理标志
    // 坐标系：右手系，X-Right，Y-Up (Assimp 默认)
    // 三角形环绕：CCW (Assimp 默认)
    // UV 原点：左上角 (通过 FlipUVs)
    // 矩阵存储：row-major (Assimp 默认，转换时处理)
    constexpr unsigned int flags = aiProcess_CalcTangentSpace |         // 生成切线空间
                                   aiProcess_JoinIdenticalVertices |    // 去重顶点，生成索引
                                   aiProcess_Triangulate |              // 三角化
                                   aiProcess_GenNormals |               // 生成法线（如果没有）
                                   aiProcess_SortByPType |              // 按图元类型排序
                                   aiProcess_FlipUVs;                   // UV 翻转为左上角原点

    // 加载场景
    ai_scene_ = importer_->ReadFile(path.string(), flags);

    if (!ai_scene_ || (ai_scene_->mFlags & AI_SCENE_FLAGS_INCOMPLETE) || !ai_scene_->mRootNode)
    {
        error_msg_ = std::format("Assimp error: {}", importer_->GetErrorString());
        return false;
    }

    // 处理材质
    scene_data_.materials.reserve(ai_scene_->mNumMaterials);
    for (unsigned int i = 0; i < ai_scene_->mNumMaterials; ++i)
    {
        scene_data_.materials.emplace_back();
        process_material(ai_scene_->mMaterials[i], scene_data_.materials.back());
    }

    // 处理 Mesh
    scene_data_.meshes.reserve(ai_scene_->mNumMeshes);
    for (unsigned int i = 0; i < ai_scene_->mNumMeshes; ++i)
    {
        scene_data_.meshes.emplace_back();
        process_mesh(ai_scene_->mMeshes[i], scene_data_.meshes.back());
    }

    // 处理节点树
    process_nodes(ai_scene_->mRootNode);

    is_loaded_ = true;
    return true;
}

void SceneImporter::clear()
{
    scene_data_.clear();
    ai_scene_ = nullptr;
    is_loaded_ = false;
    error_msg_.clear();

    // 重置 Importer（释放之前加载的场景）
    importer_ = std::make_unique<Assimp::Importer>();
}

//=============================================================================
// 节点处理
//=============================================================================

void SceneImporter::process_nodes(const aiNode* root_node)
{
    if (!root_node)
        return;

    // BFS 遍历节点树
    std::deque<std::pair<const aiNode*, aiMatrix4x4>> queue;
    queue.emplace_back(root_node, aiMatrix4x4());    // 根节点，单位矩阵

    while (!queue.empty())
    {
        auto [node, parent_transform] = queue.front();
        queue.pop_front();

        // 处理当前节点
        process_node(node, parent_transform);

        // 计算当前累积变换
        aiMatrix4x4 current_transform = parent_transform * node->mTransformation;

        // 将子节点加入队列
        for (unsigned int i = 0; i < node->mNumChildren; ++i)
        {
            queue.emplace_back(node->mChildren[i], current_transform);
        }
    }
}

void SceneImporter::process_node(const aiNode* node, const aiMatrix4x4& parent_transform)
{
    if (!node)
        return;

    InstanceData instance;

    // 名称
    instance.name = node->mName.C_Str();

    // 世界变换矩阵 (Assimp row-major -> 我们 column-major)
    aiMatrix4x4 world = parent_transform * node->mTransformation;

    // 转换为列主序
    // Assimp: a1-a4 是第1行
    // 我们: m[0-3] 是第1列
    instance.world_transform[0] = world.a1;
    instance.world_transform[1] = world.b1;
    instance.world_transform[2] = world.c1;
    instance.world_transform[3] = world.d1;

    instance.world_transform[4] = world.a2;
    instance.world_transform[5] = world.b2;
    instance.world_transform[6] = world.c2;
    instance.world_transform[7] = world.d2;

    instance.world_transform[8] = world.a3;
    instance.world_transform[9] = world.b3;
    instance.world_transform[10] = world.c3;
    instance.world_transform[11] = world.d3;

    instance.world_transform[12] = world.a4;
    instance.world_transform[13] = world.b4;
    instance.world_transform[14] = world.c4;
    instance.world_transform[15] = world.d4;

    // Mesh 和材质引用
    instance.mesh_indices.reserve(node->mNumMeshes);
    instance.material_indices.reserve(node->mNumMeshes);

    for (unsigned int i = 0; i < node->mNumMeshes; ++i)
    {
        unsigned int mesh_idx = node->mMeshes[i];
        instance.mesh_indices.push_back(mesh_idx);
        instance.material_indices.push_back(ai_scene_->mMeshes[mesh_idx]->mMaterialIndex);
    }

    scene_data_.instances.push_back(std::move(instance));
}

//=============================================================================
// Mesh 处理
//=============================================================================

void SceneImporter::process_mesh(const aiMesh* mesh, MeshData& out_mesh)
{
    if (!mesh)
        return;

    const unsigned int vertex_count = mesh->mNumVertices;
    const unsigned int face_count = mesh->mNumFaces;

    out_mesh.reserve(vertex_count, face_count);

    // 位置
    out_mesh.positions.resize(static_cast<size_t>(vertex_count) * 3);
    for (unsigned int i = 0; i < vertex_count; ++i)
    {
        out_mesh.positions[i * 3 + 0] = mesh->mVertices[i].x;
        out_mesh.positions[i * 3 + 1] = mesh->mVertices[i].y;
        out_mesh.positions[i * 3 + 2] = mesh->mVertices[i].z;
    }

    // 法线
    if (mesh->HasNormals())
    {
        out_mesh.normals.resize(static_cast<size_t>(vertex_count) * 3);
        for (unsigned int i = 0; i < vertex_count; ++i)
        {
            out_mesh.normals[i * 3 + 0] = mesh->mNormals[i].x;
            out_mesh.normals[i * 3 + 1] = mesh->mNormals[i].y;
            out_mesh.normals[i * 3 + 2] = mesh->mNormals[i].z;
        }
    }

    // 切线和副切线
    if (mesh->HasTangentsAndBitangents())
    {
        out_mesh.tangents.resize(static_cast<size_t>(vertex_count) * 3);
        out_mesh.bitangents.resize(static_cast<size_t>(vertex_count) * 3);
        for (unsigned int i = 0; i < vertex_count; ++i)
        {
            out_mesh.tangents[i * 3 + 0] = mesh->mTangents[i].x;
            out_mesh.tangents[i * 3 + 1] = mesh->mTangents[i].y;
            out_mesh.tangents[i * 3 + 2] = mesh->mTangents[i].z;

            out_mesh.bitangents[i * 3 + 0] = mesh->mBitangents[i].x;
            out_mesh.bitangents[i * 3 + 1] = mesh->mBitangents[i].y;
            out_mesh.bitangents[i * 3 + 2] = mesh->mBitangents[i].z;
        }
    }

    // UV (只取第一套)
    out_mesh.uvs.resize(static_cast<size_t>(vertex_count) * 2, 0.0f);
    if (mesh->HasTextureCoords(0))
    {
        for (unsigned int i = 0; i < vertex_count; ++i)
        {
            out_mesh.uvs[i * 2 + 0] = mesh->mTextureCoords[0][i].x;
            out_mesh.uvs[i * 2 + 1] = mesh->mTextureCoords[0][i].y;
        }
    }

    // 索引
    out_mesh.indices.reserve(static_cast<size_t>(face_count) * 3);
    for (unsigned int i = 0; i < face_count; ++i)
    {
        const aiFace& face = mesh->mFaces[i];
        // 三角化后每个面应该有 3 个顶点
        if (face.mNumIndices == 3)
        {
            out_mesh.indices.push_back(face.mIndices[0]);
            out_mesh.indices.push_back(face.mIndices[1]);
            out_mesh.indices.push_back(face.mIndices[2]);
        }
    }
}

//=============================================================================
// 材质处理
//=============================================================================

void SceneImporter::process_material(const aiMaterial* material, MaterialData& out_material) const
{
    if (!material)
        return;

    // 名称
    aiString name;
    if (material->Get(AI_MATKEY_NAME, name) == AI_SUCCESS)
    {
        out_material.name = name.C_Str();
    }

    // 基础颜色
    aiColor4D color;
    if (material->Get(AI_MATKEY_COLOR_DIFFUSE, color) == AI_SUCCESS)
    {
        out_material.base_color[0] = color.r;
        out_material.base_color[1] = color.g;
        out_material.base_color[2] = color.b;
        out_material.base_color[3] = color.a;
    }

    // 粗糙度
    ai_real roughness = 0.5f;
    if (material->Get(AI_MATKEY_ROUGHNESS_FACTOR, roughness) == AI_SUCCESS)
    {
        out_material.roughness = roughness;
    }

    // 金属度
    ai_real metallic = 0.0f;
    if (material->Get(AI_MATKEY_REFLECTIVITY, metallic) == AI_SUCCESS)
    {
        out_material.metallic = metallic;
    }

    // 自发光
    aiColor4D emissive;
    if (material->Get(AI_MATKEY_COLOR_EMISSIVE, emissive) == AI_SUCCESS)
    {
        out_material.emissive[0] = emissive.r;
        out_material.emissive[1] = emissive.g;
        out_material.emissive[2] = emissive.b;
        out_material.emissive[3] = emissive.a;
    }

    // 不透明度
    ai_real opacity = 1.0f;
    if (material->Get(AI_MATKEY_OPACITY, opacity) == AI_SUCCESS)
    {
        out_material.opacity = opacity;
    }

    // 纹理路径辅助函数
    auto get_texture_path = [&](aiTextureType type) -> std::string {
        if (material->GetTextureCount(type) == 0)
            return {};

        aiString tex_path;
        if (material->GetTexture(type, 0, &tex_path) == AI_SUCCESS)
        {
            // 转换为绝对路径
            std::filesystem::path full_path = dir_ / tex_path.C_Str();
            return full_path.string();
        }
        return {};
    };

    out_material.diffuse_map = get_texture_path(aiTextureType_DIFFUSE);
    out_material.normal_map = get_texture_path(aiTextureType_NORMALS);
}

}    // namespace truvixx