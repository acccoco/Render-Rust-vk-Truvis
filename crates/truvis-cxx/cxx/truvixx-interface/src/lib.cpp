#include "TruvixxInterface/lib.hpp"

#include "TruvixxAssimp/scene_loader.hpp"
#include "TruvixxAssimp/c_data_define.hpp"

#include <iostream>


void* load_scene(const char* mesh_path)
{
    const auto loader = new truvis::SceneLoader(mesh_path);
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
    delete static_cast<truvis::SceneLoader*>(loader);
}

size_t get_mesh_cnt(void* loader)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_geometry_count() : 0;
}

size_t get_mat_cnt(void* loader)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_material_count() : 0;
}

size_t get_instance_cnt(void* loader)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_instance_count() : 0;
}

float* get_pos_buffer(void* loader, const size_t mesh_idx, size_t* vertex_cnt)
{
    const auto scene_loader = static_cast<truvis::SceneLoader*>(loader);
    return scene_loader->get_position(mesh_idx, *vertex_cnt);
}

const CxxInstance* get_instance(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_instance(idx) : nullptr;
}

const CxxRasterGeometry* get_mesh(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_geometry(idx) : nullptr;
}

const CxxMaterial* get_mat(void* loader, size_t idx)
{
    return loader ? static_cast<truvis::SceneLoader*>(loader)->get_material(idx) : nullptr;
}