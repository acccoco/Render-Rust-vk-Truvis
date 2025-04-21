#include "../include/lib.hpp"
#include <assimp/Importer.hpp>
#include <assimp/scene.h>
#include <assimp/postprocess.h>


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
    const aiScene* scene = importer.ReadFile("path/to/your/file.obj", aiProcess_Triangulate | aiProcess_FlipUVs);

    if (!scene || scene->mFlags & AI_SCENE_FLAGS_INCOMPLETE || !scene->mRootNode)
    {
        return -1;
    }


    countVertices(scene->mRootNode, scene, totalVertices);

    return totalVertices;
}
