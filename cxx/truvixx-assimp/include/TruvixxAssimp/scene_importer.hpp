#pragma once

#include "TruvixxAssimp/scene_data.hpp"
#include "TruvixxAssimp/truvixx_assimp.export.h"

#include <filesystem>
#include <memory>
#include <string>
#include <assimp/scene.h>


namespace Assimp
{
class Importer;
}

namespace truvixx
{

/// 场景导入器
/// 使用 Assimp 加载 3D 场景文件，转换为 SOA 格式的 SceneData
///
/// 用法:
/// ```cpp
/// SceneImporter importer;
/// if (importer.load("scene.gltf")) {
///     const SceneData& data = importer.scene();
///     // 使用数据...
/// }
/// ```
class TRUVIXX_ASSIMP_API SceneImporter
{
public:
    SceneImporter();
    ~SceneImporter();

    // 禁止拷贝和移动 (持有 Assimp::Importer)
    SceneImporter(const SceneImporter&) = delete;
    SceneImporter& operator=(const SceneImporter&) = delete;
    SceneImporter(SceneImporter&&) = delete;
    SceneImporter& operator=(SceneImporter&&) = delete;

    /// 加载场景文件
    /// @param path 场景文件路径
    /// @return 成功返回 true
    [[nodiscard]] bool load(const std::filesystem::path& path);

    /// 获取加载后的场景数据 (只读引用)
    [[nodiscard]] const SceneData& scene() const noexcept { return scene_data_; }

    /// 获取最后的错误信息
    [[nodiscard]] const std::string& error() const noexcept { return error_msg_; }

    /// 是否已成功加载场景
    [[nodiscard]] bool is_loaded() const noexcept { return is_loaded_; }

    /// 清空已加载的数据
    void clear();

private:
    /// 处理场景树中的所有节点
    void process_nodes(const aiNode* root_node);

    /// 处理单个节点
    void process_node(const aiNode* node, const aiMatrix4x4& parent_transform);

    /// 处理 Mesh
    void process_mesh(const aiMesh* mesh, MeshData& out_mesh);

    /// 处理材质
    void process_material(const aiMaterial* material, MaterialData& out_material) const;

private:
    std::unique_ptr<Assimp::Importer> importer_;    ///< Assimp 导入器，持有 ai_scene 生命周期
    const aiScene* ai_scene_ = nullptr;             ///< Assimp 场景 (由 importer_ 管理)

    SceneData scene_data_;         ///< 转换后的场景数据
    std::filesystem::path dir_;    ///< 场景文件所在目录
    std::string error_msg_;        ///< 错误信息
    bool is_loaded_ = false;       ///< 加载状态
};

}    // namespace truvixx