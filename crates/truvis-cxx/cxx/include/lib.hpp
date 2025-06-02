#pragma once
#include "public/scene_loader/c_data_define.hpp"

#ifdef BUILDING_DLL    // 该 macro 由 cmake 定义
#define DLL_API __declspec(dllexport)
#else
// dllimport 不是必须的，因为有 导入库 .lib 告诉连接器哪些符号需要动态链接
#define DLL_API __declspec(dllimport)
#endif


extern "C" {
DLL_API void* load_scene(const char* mesh_path);
DLL_API void free_scene(void* loader);

DLL_API size_t get_mesh_cnt(void* loader);
DLL_API size_t get_mat_cnt(void* loader);
DLL_API size_t get_instance_cnt(void* loader);

DLL_API const CxxInstance* get_instance(void* loader, size_t idx);
DLL_API const CxxRasterGeometry* get_mesh(void* loader, size_t idx);
DLL_API const CxxMaterial* get_mat(void* loader, size_t idx);
}