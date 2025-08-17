#include "private/scene_loader/data_convert.hpp"
#include "private/scene_loader/scene_loader.hpp"

#include <assimp/matrix4x4.h>
#include <cassert>
#include <iostream>
#include <format>
#include <assimp/postprocess.h>
#include <assimp/scene.h>
#include <deque>

namespace truvis
{
bool SceneLoader::load_scene()
{
    // 检查 mesh_path 和 dir_path 是否存在
    if (!std::filesystem::exists(this->mesh_path_) || !std::filesystem::is_regular_file(this->mesh_path_)
        || !std::filesystem::exists(this->dir_path_))
    {
        std::cerr << std::format("Mesh file {} or dir {} not found", this->mesh_path_.string(),
                                 this->dir_path_.string())
                  << "\n";
        return false;
    }

    // importer 析构时，会自动回收资源
    Assimp::Importer assimp_impoter;

    // Assimp 导入的后处理操作
    // 默认坐标系是右手系，X-Right，Y-Up，可以通过 MakeLeftHanded 标志来修改
    // 默认三角形环绕方向是 CCW，可以通过 FlipWindingOrder 标志来修改
    // 默认 UV 以左下角为原点，可以通过 FlipUVs 标志修改为左上角
    // 默认矩阵采用 row major 的存储方式
    constexpr auto post_process_flags =
            aiProcess_CalcTangentSpace           // 如果顶点具有法线属性，自动生成 tangent space 属性
            | aiProcess_JoinIdenticalVertices    // 确保 index buffer 存在，且每个顶点都是不重复的
            | aiProcess_Triangulate              // 将所有的面三角化
            | aiProcess_GenNormals               // 如果没有法线，自动生成面法线
            | aiProcess_SortByPType              // 在三角化之后发生，可以去除 point 和 line
            | aiProcess_FlipUVs;


    // 载入模型文件
    // 参考 Assimp 的文档：https://assimp-docs.readthedocs.io/en/v5.1.0/usage/use_the_lib.html
    this->ai_scene_ = assimp_impoter.ReadFile(mesh_path_.string(), post_process_flags);
    if (!this->ai_scene_ || (this->ai_scene_->mFlags & AI_SCENE_FLAGS_INCOMPLETE) || !this->ai_scene_->mRootNode)
    {
        std::cout << std::format("{}", assimp_impoter.GetErrorString()) << "\n";
        return false;
    }

    // 处理所有的节点 -> instance
    std::deque<std::tuple<aiNode*, aiMatrix4x4>> node_queue;
    node_queue.emplace_back(this->ai_scene_->mRootNode, aiMatrix4x4());
    while (!node_queue.empty())
    {
        auto [node, parent_transform] = node_queue.front();
        node_queue.pop_front();

        // 处理当前节点
        this->instances_.emplace_back();
        auto& instance = this->instances_.back();
        (void) process_node(instance, *node, parent_transform);

        // 处理子节点 - 需要累积变换矩阵
        aiMatrix4x4 current_transform = parent_transform * node->mTransformation;
        for (uint32_t i = 0; i < node->mNumChildren; ++i)
        {
            node_queue.emplace_back(node->mChildren[i], current_transform);
        }
    }

    // 处理所有的材质
    this->materials_.reserve(ai_scene_->mNumMaterials);
    for (uint32_t i = 0; i < ai_scene_->mNumMaterials; ++i)
    {
        const auto ai_mat = ai_scene_->mMaterials[i];
        this->materials_.emplace_back();
        auto& mat = this->materials_.back();
        (void) process_material(mat, *ai_mat);
    }

    // 处理所有的 mesh
    this->geometries_.reserve(ai_scene_->mNumMeshes);
    for (uint32_t i = 0; i < ai_scene_->mNumMeshes; ++i)
    {
        const auto ai_mesh = ai_scene_->mMeshes[i];
        this->geometries_.emplace_back();
        auto& geo = this->geometries_.back();
        (void) process_geometry(geo, *ai_mesh);
    }

    return true;
}


bool SceneLoader::process_node(CxxInstance& instance, const aiNode& ai_node, const aiMatrix4x4& parent_transform) const
{
    // 节点名称
    {
        strncpy_s(instance.name, PATH_BUFFER_SIZE, ai_node.mName.C_Str(), _TRUNCATE);
    }

    instance.world_transform = DataConvert::mat4(parent_transform * ai_node.mTransformation);
    instance.init(ai_node.mNumMeshes);

    for (unsigned int i = 0; i < instance.mesh_cnt(); ++i)
    {
        instance.mesh_indices()[i] = ai_node.mMeshes[i];
        instance.mat_indices()[i] = ai_scene_->mMeshes[ai_node.mMeshes[i]]->mMaterialIndex;
    }

    return true;
}

bool SceneLoader::process_material(CxxMaterial& material, const aiMaterial& ai_mat) const
{
    // 材质名称
    {
        aiString out_name;
        ai_mat.Get(AI_MATKEY_NAME, out_name);
        strncpy_s(material.name, PATH_BUFFER_SIZE, out_name.C_Str(), _TRUNCATE);
    }

    // 提取出各种颜色
    {

        {
            aiColor4D out_color = {};
            ai_mat.Get(AI_MATKEY_COLOR_DIFFUSE, out_color);
            material.base_color = DataConvert::vec4(out_color);
        }
        {
            // 这个 Key 有点奇怪
            ai_real out_real = {};
            ai_mat.Get(AI_MATKEY_REFLECTIVITY, out_real);
            material.metallic_factor = out_real;
        }
        {
            ai_real out_real = {};
            ai_mat.Get(AI_MATKEY_ROUGHNESS_FACTOR, out_real);
            material.roughness_factor = out_real;
        }
        {
            aiColor4D out_color = {};
            ai_mat.Get(AI_MATKEY_COLOR_EMISSIVE, out_color);
            material.emissive_color = DataConvert::vec4(out_color);
        }
        {
            ai_real out_real = {};
            ai_mat.Get(AI_MATKEY_OPACITY, out_real);
            material.opaque_factor = out_real;
        }
    }

    // 提取出各种纹理
    {
        const auto get_texture = [&](const aiTextureType tex_type, char* dest, const size_t max_len) {
            if (ai_mat.GetTextureCount(tex_type) == 0)
            {
                dest[0] = '\0';    // 空字符串
                return;
            }

            aiString out_path;    // 获取到的是相对路径
            ai_mat.GetTexture(tex_type, 0, &out_path);

            const auto tex_path = dir_path_ / out_path.C_Str();
            const std::string path_str = tex_path.string();

            // 安全地复制字符串，使用 strncpy_s 替代 strncpy
            strncpy_s(dest, max_len, path_str.c_str(), _TRUNCATE);
        };

        // 复制各种纹理路径到材质结构中
        get_texture(aiTextureType_DIFFUSE, material.diffuse_map, PATH_BUFFER_SIZE);
        get_texture(aiTextureType_NORMALS, material.normal_map, PATH_BUFFER_SIZE);
    }

    return true;
}

bool SceneLoader::process_geometry(CxxRasterGeometry& geometry, const aiMesh& ai_mesh)
{
    geometry.init(ai_mesh.mNumVertices, ai_mesh.mNumFaces);

    // 处理面
    for (unsigned int i = 0; i < geometry.face_cnt(); ++i)
    {
        // 通过 Assimp 的 post-process 保证了这里的 face 都是 triangle
        assert(ai_mesh.mFaces[i].mNumIndices == 3);
        geometry.faces()[i] = CxxTriangleFace{
                .a = ai_mesh.mFaces[i].mIndices[0],
                .b = ai_mesh.mFaces[i].mIndices[1],
                .c = ai_mesh.mFaces[i].mIndices[2],
        };
    }

    // 通过 Assimp 的 post-process 保证了一定会有 normal，tangent
    assert(ai_mesh.HasNormals() && ai_mesh.HasTangentsAndBitangents());

    // 处理顶点
    for (unsigned int i = 0; i < geometry.vertex_cnt(); ++i)
    {
        // position
        geometry.vertices()[i].position = DataConvert::vec3(ai_mesh.mVertices[i]);

        // normal
        geometry.vertices()[i].normal = DataConvert::vec3(ai_mesh.mNormals[i]);

        // tangent and biTangent
        geometry.vertices()[i].tangent = DataConvert::vec3(ai_mesh.mTangents[i]);
        geometry.vertices()[i].bitangent = DataConvert::vec3(ai_mesh.mBitangents[i]);

        // 默认的 UV 值
        geometry.vertices()[i].uv = CxxVec2f{.x = 0.0f, .y = 0.0f};
    }

    // uv: Assimp 最多支持 8 套 uv。我们只需要第一套就好
    if (ai_mesh.HasTextureCoords(0))
    {
        for (unsigned int i = 0; i < geometry.vertex_cnt(); ++i)
        {
            geometry.vertices()[i].uv = DataConvert::vec2(ai_mesh.mTextureCoords[0][i]);
        }
    }

    return true;
}

}    // namespace truvis