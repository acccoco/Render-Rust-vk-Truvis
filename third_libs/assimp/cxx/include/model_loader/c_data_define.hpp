#pragma once


/// 4 字节对齐
struct alignas(4) CxxVec2f
{
    float x;
    float y;
};

/// 4 字节对齐
struct alignas(4) CxxVec3f
{
    float x;
    float y;
    float z;
};

/// 4 字节对齐
struct alignas(4) CxxVec4f
{
    float x;
    float y;
    float z;
    float w;
};

/// 4x4 矩阵结构体 (列主序)
struct alignas(4) CxxMat4f
{
    float m[16]; // m[0]..m[3] 是第一列，以此类推
};

/// 三角形面结构体
struct alignas(4) CxxTriangleFace
{
    unsigned int a;
    unsigned int b;
    unsigned int c;
};

/// 顶点结构体
struct alignas(4) CxxVertex3D
{
    // 坐标系：右手系，X-Right，Y-Up
    CxxVec3f position;
    CxxVec3f normal;
    CxxVec3f tangent;
    CxxVec3f bitangent;
    CxxVec2f uv;
};

/// 适合光栅化的几何体结构体，以 AoS(Array of Struct) 的形式组织
struct CxxRasterGeometry
{
    CxxRasterGeometry() = default;

    ~CxxRasterGeometry()
    {
        delete[] vertex_array_;
        delete[] face_array_;
        vertex_array_ = nullptr;
        face_array_ = nullptr;
    }

    /// 移动构造会在 vector 扩容的时候被调用，避免调用 destructor 导致内存异常释放
    CxxRasterGeometry(CxxRasterGeometry&& other) noexcept
        : vertex_cnt_(other.vertex_cnt_),
          vertex_array_(other.vertex_array_),
          face_cnt_(other.face_cnt_),
          face_array_(other.face_array_)
    {
        other.vertex_array_ = nullptr;
        other.face_array_ = nullptr;
    }

    void init(const unsigned int vertex_cnt, const unsigned int face_cnt)
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

    [[nodiscard]] unsigned int vertex_cnt() const { return vertex_cnt_; }
    [[nodiscard]] unsigned int face_cnt() const { return face_cnt_; }
    [[nodiscard]] CxxVertex3D* vertices() const { return vertex_array_; }
    [[nodiscard]] CxxTriangleFace* faces() const { return face_array_; }

private:
    unsigned int vertex_cnt_ = 0;
    CxxVertex3D* vertex_array_ = nullptr;

    unsigned int face_cnt_ = 0;
    CxxTriangleFace* face_array_ = nullptr;
};

/// 材质结构体
struct CxxMaterial
{
    CxxVec4f ambient;
    CxxVec4f diffuse;
    CxxVec4f specular;
    CxxVec4f emission;

    /// 字符串使用 C 风格字符数组，确保以 null 结尾
    char diffuse_map[256];
    char ambient_map[256];
    char emissive_map[256];
    char specular_map[256];
};

struct CxxInstance
{
    CxxInstance() = default;

    /// 移动构造会在 vector 扩容的时候被调用，避免调用 destructor 导致内存异常释放
    CxxInstance(CxxInstance&& other) noexcept
        : world_transform(other.world_transform),
          mesh_cnt_(other.mesh_cnt_),
          mat_indices_(other.mat_indices_),
          mesh_indices_(other.mesh_indices_)
    {
        other.mat_indices_ = nullptr;
        other.mesh_indices_ = nullptr;
    }


    ~CxxInstance()
    {
        delete[] mat_indices_;
        delete[] mesh_indices_;
        mat_indices_ = nullptr;
        mesh_indices_ = nullptr;
    }

    /// mesh 有可能是 0 个，需要格外小心
    void init(const unsigned int geometry_cnt)
    {
        mesh_cnt_ = geometry_cnt;
        if (mesh_cnt_ != 0)
        {
            this->mat_indices_ = new unsigned int[geometry_cnt];
            this->mesh_indices_ = new unsigned int[geometry_cnt];
        }
    }

    [[nodiscard]] unsigned int* mat_indices() const { return mat_indices_; }
    [[nodiscard]] unsigned int* mesh_indices() const { return mesh_indices_; }
    [[nodiscard]] unsigned int mesh_cnt() const { return mesh_cnt_; }

    /// 坐标系：右手系，X-Right，Y-Up
    CxxMat4f world_transform = {};

private:
    /// 几何体索引数量
    unsigned int mesh_cnt_ = 0;

    unsigned int* mat_indices_ = nullptr;

    unsigned int* mesh_indices_ = nullptr;
};