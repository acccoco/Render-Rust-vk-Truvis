#include "public/scene_loader/c_data_define.hpp"

#include <string.h>

CxxRasterGeometry::~CxxRasterGeometry()
{
    delete[] vertex_array_;
    delete[] face_array_;
    vertex_array_ = nullptr;
    face_array_ = nullptr;
}


CxxRasterGeometry::CxxRasterGeometry(CxxRasterGeometry&& other) noexcept
    : vertex_array_(other.vertex_array_),
      face_array_(other.face_array_),
      vertex_cnt_(other.vertex_cnt_),
      face_cnt_(other.face_cnt_)
{
    other.vertex_array_ = nullptr;
    other.face_array_ = nullptr;
    other.vertex_cnt_ = 0;
    other.face_cnt_ = 0;
}


void CxxRasterGeometry::init(const unsigned int vertex_cnt, const unsigned int face_cnt)
{
    this->vertex_cnt_ = vertex_cnt;
    this->face_cnt_ = face_cnt;

    if (vertex_cnt != 0)
    {
        this->vertex_array_ = new CxxVertex3D[vertex_cnt];
    }
    if (face_cnt != 0)
    {
        this->face_array_ = new CxxTriangleFace[face_cnt];
    }
}


CxxInstance::CxxInstance(CxxInstance&& other) noexcept
    : world_transform(other.world_transform),
      mat_indices_(other.mat_indices_),
      mesh_indices_(other.mesh_indices_),
      mesh_cnt_(other.mesh_cnt_)

{
    // 将 other 的 name 成员复制到当前实例
    strncpy_s(this->name, PATH_BUFFER_SIZE, other.name, _TRUNCATE);
    other.mat_indices_ = nullptr;
    other.mesh_indices_ = nullptr;
}


CxxInstance::~CxxInstance()
{
    delete[] mat_indices_;
    delete[] mesh_indices_;
    mat_indices_ = nullptr;
    mesh_indices_ = nullptr;
}


void CxxInstance::init(const unsigned int geometry_cnt)
{
    mesh_cnt_ = geometry_cnt;
    if (mesh_cnt_ != 0)
    {
        this->mat_indices_ = new unsigned int[geometry_cnt];
        this->mesh_indices_ = new unsigned int[geometry_cnt];
    }
}
