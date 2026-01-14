#include "./payload.slangi"
#include "lib/bindless_op.slangi"

/// 漫反射颜色采样
/// 根据材质的 diffuse_map 或 base_color 返回漫反射颜色
[shader("callable")]
void diffuse_callable(inout MatPayload payload)
{
    PBRMaterial* mat = gpu_scene.get_material(payload.instance_id, payload.geometry_id);
    if (bindless_srv::is_valid(mat.diffuse_map))
    {
        payload.out_color = bindless_srv::sample_level(mat.diffuse_map, payload.uv, mat.diffuse_map_sampler_type, 0.0).xyz;
    }
    else
    {
        payload.out_color = mat.base_color;
    }
}
