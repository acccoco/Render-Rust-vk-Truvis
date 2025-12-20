#include "./phong.slangi"
#include "bindless.slangi"
#include "pass/raster.slangi"
#include "scene.slangi"

struct PsInput
{
    CoarseVertex coarse_vertex : CoarseVertex;
};

struct PsOutput
{
    [[vk::location(0)]]
    float4 color : SV_TARGET0;
};

[[vk::push_constant]]
raster::PushConstants push_const;

[shader("pixel")]
PsOutput main(PsInput input)
{
    const float3 normal = normalize(input.coarse_vertex.frag_normal);

    PerFrameData *frame_data = push_const.frame_data;
    GPUScene* scene = push_const.scene;
    Instance* instance = scene.all_instances + push_const.instance_idx;
    PBRMaterial* mat = scene->get_material(push_const.instance_idx, push_const.submesh_idx);

    const uint light_cnt = scene.point_light_count.x;

    const float2 delta_uv = frac(frame_data.time_ms / 1000.0f);
    const float2 uv = input.coarse_vertex.uv + delta_uv;
    const float4 object_color = mat.diffuse_map.sample(uv);

    float3 light_term = float3(0.0, 0.0, 0.0);
    for (uint i = 0; i < light_cnt; i++)
    {
        const PointLight point_light = scene.point_lights[i];
        light_term += point_light.phong_light(frame_data.camera_pos, input.coarse_vertex.world_pos, normal, object_color);
    }

    const float3 min_color = object_color.xyz * 0.5;

    PsOutput output = (PsOutput)0;
    output.color = float4(max(light_term, min_color), 1.0f);
    return output;
}
