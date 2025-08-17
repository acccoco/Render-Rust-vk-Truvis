#pragma once
#include "public/scene_loader/c_data_define.hpp"
#include "dll_config.hpp"

extern "C" {

#pragma region assimp
/// 加载场景文件
///
/// @return 返回一个指向加载器的指针，调用者需要在不需要时调用 free_scene() 释放资源
DLL_API void* load_scene(const char* mesh_path);

/// 释放加载器资源
/// @param loader 指向加载器的指针，必须是通过 load_scene() 返回的指针
DLL_API void free_scene(void* loader);

DLL_API size_t get_mesh_cnt(void* loader);
DLL_API size_t get_mat_cnt(void* loader);
DLL_API size_t get_instance_cnt(void* loader);

DLL_API float* get_pos_buffer(void* loader, size_t mesh_idx, size_t* vertex_cnt);

DLL_API const CxxInstance* get_instance(void* loader, size_t idx);
DLL_API const CxxRasterGeometry* get_mesh(void* loader, size_t idx);
DLL_API const CxxMaterial* get_mat(void* loader, size_t idx);
#pragma endregion
}