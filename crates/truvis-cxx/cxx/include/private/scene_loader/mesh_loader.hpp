#pragma once

#include <assimp/matrix4x4.h>
#include <filesystem>
#include <vector>
#include <assimp/material.h>
#include <assimp/mesh.h>
#include <assimp/scene.h>
#include <public/scene_loader/c_data_define.hpp>


namespace truvis
{

/// 场景加载器，使用 Assimp 库加载场景文件
struct SceneLoader
{
    explicit SceneLoader(const std::filesystem::path& mesh_path)
        : mesh_path_(mesh_path),
          dir_path_(mesh_path.parent_path())
    {}
    ~SceneLoader() = default;
    SceneLoader(const SceneLoader&) = delete;
    SceneLoader(SceneLoader&&) = delete;
    SceneLoader& operator=(const SceneLoader&) = delete;
    SceneLoader& operator=(SceneLoader&&) = delete;

    /// 加载场景文件，将所有的实例、几何体和材质信息提取出来
    bool load_scene();

#pragma region getter
    [[nodiscard]] const CxxInstance* get_instance(const size_t index) const
    {
        return index < instances_.size() ? &instances_[index] : nullptr;
    }
    [[nodiscard]] const CxxRasterGeometry* get_geometry(const size_t index) const
    {
        return index < geometries_.size() ? &geometries_[index] : nullptr;
    }

    [[nodiscard]] const CxxMaterial* get_material(const size_t index) const
    {
        return index < materials_.size() ? &materials_[index] : nullptr;
    }

    [[nodiscard]] size_t get_instance_count() const { return instances_.size(); }
    [[nodiscard]] size_t get_geometry_count() const { return geometries_.size(); }
    [[nodiscard]] size_t get_material_count() const { return materials_.size(); }
#pragma endregion

private:
    /// 处理一个节点，节点中包括多个 mesh，不考虑子节点
    [[nodiscard]]
    bool process_node(CxxInstance& instance, const aiNode& ai_node, const aiMatrix4x4& parent_transform) const;

    /// 从 aiMaterial 中提取 material 的信息
    [[nodiscard]]
    bool process_material(CxxMaterial& material, const aiMaterial& ai_mat) const;

    /// 从 aiMesh 中提取几何信息
    [[nodiscard]]
    static bool process_geometry(CxxRasterGeometry& geometry, const aiMesh& ai_mesh);

private:
    const std::filesystem::path mesh_path_;    // mesh 文件对应的路径
    const std::filesystem::path dir_path_;     // mesh 文件所在的文件夹，形式："xx/xxx"

    std::vector<CxxInstance> instances_;           // 所有的实例
    std::vector<CxxRasterGeometry> geometries_;    // 所有的几何体
    std::vector<CxxMaterial> materials_;           // 所有的材质

    const aiScene* ai_scene_ = nullptr;
};
}    // namespace truvis