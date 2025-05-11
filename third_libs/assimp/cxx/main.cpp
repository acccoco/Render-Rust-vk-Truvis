#include <iostream>

#include "lib.hpp"

int main()
{
    auto loader = load_scene("C:/Users/bigso/OneDrive/Library/AssetsLib/model/gltf/dancing-girl/scene.gltf");
    if (!loader)
    {
        std::cerr << "Failed to load scene." << std::endl;
        return -1;
    }

    const auto mesh_cnt = get_mesh_cnt(loader);
    const auto mat_cnt = get_mat_cnt(loader);
    const auto instance_cnt = get_instance_cnt(loader);

    std::cout << "Mesh count: " << mesh_cnt << '\n';
    std::cout << "Material count: " << mat_cnt << '\n';
    std::cout << "Instance count: " << instance_cnt << '\n';

    free_scene(loader);

    return 0;
}
