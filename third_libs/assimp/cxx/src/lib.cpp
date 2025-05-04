#include "lib.hpp"
#include "model_loader/c_data_define.hpp"
#include "model_loader/mesh_loader.hpp"
#include <assimp/Importer.hpp>
#include <assimp/scene.h>
#include <assimp/postprocess.h>
#include <iostream>


unsigned int totalVertices = 0;

// Helper function to count vertices in a node
void countVertices(const aiNode* node, const aiScene* scene, unsigned int& totalVertices)
{
    for (unsigned int i = 0; i < node->mNumMeshes; i++)
    {
        const aiMesh* mesh = scene->mMeshes[node->mMeshes[i]];
        totalVertices += mesh->mNumVertices;
    }

    for (unsigned int i = 0; i < node->mNumChildren; i++)
    {
        countVertices(node->mChildren[i], scene, totalVertices);
    }
}


unsigned int get_vert_cnts()
{
    Assimp::Importer importer;
    const aiScene* scene = importer.ReadFile("D:\\code\\Render-Rust-vk-Truvis\\assets\\obj\\spot.obj",
                                             aiProcess_Triangulate | aiProcess_FlipUVs);

    if (!scene || scene->mFlags & AI_SCENE_FLAGS_INCOMPLETE || !scene->mRootNode)
    {
        return -1;
    }


    countVertices(scene->mRootNode, scene, totalVertices);

    return totalVertices;
}


void* load_scene(const char* mesh_path)
{
    const auto loader = new truvis::MeshLoader(mesh_path);
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
    delete static_cast<truvis::MeshLoader*>(loader);
}

size_t get_mesh_cnt(void* loader)
{
    return loader ? static_cast<truvis::MeshLoader*>(loader)->get_geometry_count() : 0;
}

size_t get_mat_cnt(void* loader)
{
    return loader ? static_cast<truvis::MeshLoader*>(loader)->get_material_count() : 0;
}

size_t get_instance_cnt(void* loader)
{
    return loader ? static_cast<truvis::MeshLoader*>(loader)->get_instance_count() : 0;
}

const CxxInstance* get_instance(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::MeshLoader*>(loader)->get_instance(idx) : nullptr;
}

const CxxRasterGeometry* get_mesh(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::MeshLoader*>(loader)->get_geometry(idx) : nullptr;
}

const CxxMaterial* get_mat(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::MeshLoader*>(loader)->get_material(idx) : nullptr;
}