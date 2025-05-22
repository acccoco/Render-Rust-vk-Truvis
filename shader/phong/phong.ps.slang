#include "phong.slangi"
#include "../include/scene.slangi"

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
PushConstants push_const;

// TODO 可能需要 nonuniform，表示同一个 wavefront 的下标是不同的
[[vk::binding(0, 0)]]
Sampler2D<float4> textures[];

[[vk::binding(1, 0)]]
Texture2D<float4> images[];


func phong_light(camera_pos: float3, pos: float3, normal: float3, light: PointLight, object_color: float4)->float3
{
    const float3 light_dir = normalize(pos - light.pos);
    const float3 view_dir = normalize(pos - camera_pos);
    const float3 reflect_dir = normalize(reflect(light_dir, normal));
    const float3 halfway = -normalize(light_dir + view_dir);

    const float diffuse_coef = max(0.0, dot(-light_dir, normal));
    const float specular_coef = pow(max(0.0, dot(normal, halfway)), 8.0);

    const float const_term = 1.0f;
    const float linear_term = 0.09f;
    const float quadratic_term = 0.032f;
    const float distance = length(light.pos - pos);

    // FIXME attenuation
    const float light_attenuation = 10.0 / (const_term + linear_term * distance + quadratic_term * distance * distance);

    const float3 diffuse_color = object_color.rgb * diffuse_coef * light.color;
    const float3 specular_color = float3(1.f, 1.f, 1.f) * specular_coef;

    const float3 color = (diffuse_color + specular_color) * light_attenuation;
    return color;
}

[shader("pixel")]
PsOutput main(PsInput input)
{
    const float3 normal = normalize(input.coarse_vertex.frag_normal);

    PerFrameData* frame_data = push_const.frame_data;
    Scene* scene = push_const.scene;
    Instance* instance = scene.all_instances + push_const.instance_idx;
    PBRMaterial* mat = scene->get_material(push_const.instance_idx, push_const.submesh_idx);

    const uint light_cnt = scene.point_light_count.x;
    const uint diffuse_map_id = mat.diffuse_map;

    const float2 uv = input.coarse_vertex.uv;
    const float4 object_color = textures[diffuse_map_id].Sample(uv);

    float3 light_term = float3(0.0, 0.0, 0.0);
    for (uint i = 0; i < light_cnt; i++)
    {
        const PointLight point_light = scene.point_lights[i];
        light_term += phong_light(frame_data.camera_pos, input.coarse_vertex.world_pos, normal, point_light, object_color);
    }

    const float3 min_color = object_color.xyz * 0.5;

    PsOutput output = (PsOutput)0;
    output.color = float4(max(light_term, min_color), 1.0f);
    return output;
}
