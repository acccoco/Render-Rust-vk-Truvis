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
    float m[16];    // m[0]..m[3] 是第一列，以此类推
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
    ~CxxRasterGeometry();
    /// 移动构造会在 vector 扩容的时候被调用，避免调用 destructor 导致内存异常释放
    CxxRasterGeometry(CxxRasterGeometry&& other) noexcept;
    CxxRasterGeometry(const CxxRasterGeometry&) = delete;
    CxxRasterGeometry& operator=(const CxxRasterGeometry&) = delete;
    CxxRasterGeometry& operator=(CxxRasterGeometry&&) = delete;

    void init(unsigned int vertex_cnt, const unsigned int face_cnt);

    [[nodiscard]] unsigned int vertex_cnt() const { return vertex_cnt_; }
    [[nodiscard]] unsigned int face_cnt() const { return face_cnt_; }
    [[nodiscard]] CxxVertex3D* vertices() const { return vertex_array_; }
    [[nodiscard]] CxxTriangleFace* faces() const { return face_array_; }

private:
    CxxVertex3D* vertex_array_ = nullptr;
    CxxTriangleFace* face_array_ = nullptr;
    unsigned int vertex_cnt_ = 0;
    unsigned int face_cnt_ = 0;
};


constexpr static size_t PATH_BUFFER_SIZE = 256;

/// 材质结构体
struct CxxMaterial
{
    char name[PATH_BUFFER_SIZE];    // 材质名称，使用 C 风格字符数组，确保以 null 结尾

    CxxVec4f base_color;
    float roughness_factor;
    float metallic_factor;

    CxxVec4f emissive_color;

    float opaque_factor;    // 透射率，1 表示 opaque, 0 表示 transparent

    /// 字符串使用 C 风格字符数组，确保以 null 结尾
    char diffuse_map[PATH_BUFFER_SIZE];
    char normal_map[PATH_BUFFER_SIZE];
};

struct CxxInstance
{
    CxxInstance() = default;
    ~CxxInstance();
    /// 移动构造会在 vector 扩容的时候被调用，避免调用 destructor 导致内存异常释放
    CxxInstance(CxxInstance&& other) noexcept;
    CxxInstance(const CxxInstance&) = delete;
    CxxInstance& operator=(const CxxInstance&) = delete;
    CxxInstance& operator=(CxxInstance&&) = delete;

    /// mesh 有可能是 0 个，需要格外小心
    void init(unsigned int geometry_cnt);

    [[nodiscard]] unsigned int* mat_indices() const { return mat_indices_; }
    [[nodiscard]] unsigned int* mesh_indices() const { return mesh_indices_; }
    [[nodiscard]] unsigned int mesh_cnt() const { return mesh_cnt_; }

    /// 坐标系：右手系，X-Right，Y-Up
    CxxMat4f world_transform = {};
    char name[PATH_BUFFER_SIZE];    // 名称，使用 C 风格字符数组，确保以 null 结尾

private:
    unsigned int* mat_indices_ = nullptr;
    unsigned int* mesh_indices_ = nullptr;

    /// 几何体索引数量
    unsigned int mesh_cnt_ = 0;
    int _padding_ = 0;
};