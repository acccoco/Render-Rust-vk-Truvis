#pragma once
#include <assimp/Importer.hpp>
#include <model_loader/c_data_define.hpp>


namespace truvis
{
struct DataConvert
{
    /// 将 Assimp 的 vec3 转换为 C_Vec3f
    static CxxVec3f vec3(const aiVector3D& vec) { return {vec.x, vec.y, vec.z}; }

    /// 将 Assimp 的 vec2 提取为 C_Vec2f（从 aiVector3D 中提取 x 和 y）
    static CxxVec2f vec2(const aiVector3D& vec) { return {vec.x, vec.y}; }

    /// 将 Assimp 的颜色转换为 C_Vec4f
    static CxxVec4f vec4(const aiColor4D& color) { return {color.r, color.g, color.b, color.a}; }

    /// 将 Assimp 的矩阵转化为 C_Mat4f
    /// @details Assimp 的矩阵是 row-major 的，a 表示第 1 行，d 表示第 4 行
    /// @details 转换为列主序的 C_Mat4f
    static CxxMat4f mat4(const aiMatrix4x4& mat)
    {
        CxxMat4f result;
        result.m[0] = mat.a1;
        result.m[1] = mat.b1;
        result.m[2] = mat.c1;
        result.m[3] = mat.d1;    // col 1

        result.m[4] = mat.a2;
        result.m[5] = mat.b2;
        result.m[6] = mat.c2;
        result.m[7] = mat.d2;    // col 2

        result.m[8] = mat.a3;
        result.m[9] = mat.b3;
        result.m[10] = mat.c3;
        result.m[11] = mat.d3;    // col 3

        result.m[12] = mat.a4;
        result.m[13] = mat.b4;
        result.m[14] = mat.c4;
        result.m[15] = mat.d4;    // col 4
        return result;
    }
};
}    // namespace truvis