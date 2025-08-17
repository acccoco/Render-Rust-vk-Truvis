#include <iostream>

#include "lib.hpp"

int main(const int argc, char* argv[])
{
    if (argc < 2)
    {
        std::cerr << "Usage: " << argv[0] << " <path_to_scene_file>\n";
        return -1;
    }
    const auto loader = load_scene(argv[1]);
    if (!loader)
    {
        std::cerr << "Failed to load scene." << "\n";
        return -1;
    }

    const auto mesh_cnt = get_mesh_cnt(loader);
    const auto mat_cnt = get_mat_cnt(loader);
    const auto instance_cnt = get_instance_cnt(loader);

    std::cout << "Mesh count: " << mesh_cnt << '\n';
    std::cout << "Material count: " << mat_cnt << '\n';
    std::cout << "Instance count: " << instance_cnt << '\n';

    const auto print_vec4 = [](const CxxVec4f& vec) {
        std::cout << "(" << vec.x << ", " << vec.y << ", " << vec.z << ", " << vec.w << ")";
    };

    for (size_t instance_idx = 0; instance_idx < instance_cnt; ++instance_idx)
    {
        const auto instance = get_instance(loader, instance_idx);
        if (!instance)
        {
            std::cerr << "Failed to get instance at index " << instance_idx << "\n";
            continue;
        }

        auto instance_name = std::string(instance->name);
        std::cout << "Instance " << instance_idx << "(" << instance_name << ")"
                  << ": mesh count = " << instance->mesh_cnt() << "\n";
        for (size_t i = 0; i < instance->mesh_cnt(); ++i)
        {
            const auto mesh = get_mesh(loader, instance->mesh_indices()[i]);
            if (!mesh)
            {
                std::cerr << "Failed to get mesh at index " << instance->mesh_indices()[i] << "\n";
                continue;
            }
            std::cout << "  Mesh " << i << ": vertex count = " << mesh->vertex_cnt()
                      << ", face count = " << mesh->face_cnt() << "\n";

            const auto mat = get_mat(loader, instance->mat_indices()[i]);
            if (!mat)
            {
                std::cerr << "Failed to get material at index " << instance->mat_indices()[i] << "\n";
                continue;
            }
            auto mat_name = std::string(mat->name);
            std::cout << "  Material " << i << ":" << mat_name << "\n";
            std::cout << " base color = ";
            print_vec4(mat->base_color);
            std::cout << "roughness: " << mat->roughness_factor;
            std::cout << ", metallic: " << mat->metallic_factor << "\n";
            std::cout << "  Emissive color: ";
            print_vec4(mat->emissive_color);
            std::cout << ", transmission factor: " << mat->opaque_factor << "\n";

            std::cout << "base color texture: " << mat->diffuse_map << "\n"
                      << "normal texture: " << mat->normal_map << "\n";
        }
    }

    free_scene(loader);

    return 0;
}
