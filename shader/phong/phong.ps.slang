#include "phong.slangi"

struct PsInput
{
    [[vk::location(0)]]
    float3 world_pos : WORLD_POS;

    [[vk::location(1)]]
    float3 frag_normal : FRAG_NORMAL;

    [[vk::location(2)]]
    float2 uv : UV;
};

struct PsOutput
{
    [[vk::location(0)]]
    float4 color : SV_TARGET0;
};

[[vk::push_constant]]
PushConstants push_constants;

[[vk::binding(0, 0)]]
cbuffer SceneUBO
{
    float4x4 projection;
    float4x4 view;

    Light light_1;
    Light light_2;
    Light light_3;
};

[[vk::binding(0, 2)]]
cbuffer MaterialUBO
{
    float4 color;
};

func phong_light(pos: float3, normal: float3, light_pos: float3, light_color: float3)->float3
{
    const float3 light_dir = normalize(pos - light_pos);
    const float3 view_dir = normalize(pos - push_constants.camera_pos);
    const float3 reflect_dir = normalize(reflect(light_dir, normal));
    const float3 halfway = -normalize(light_dir + view_dir);

    const float diffuse_coef = max(0.0, dot(-light_dir, normal));
    const float specular_coef = pow(max(0.0, dot(normal, halfway)), 8.0);

    const float const_term = 1.0f;
    const float linear_term = 0.09f;
    const float quadratic_term = 0.032f;
    const float distance = length(light_pos - pos);

    // FIXME attenuation
    const float light_attenuation = 10.0 / (const_term + linear_term * distance + quadratic_term * distance * distance);

    // FIXME object color
    const float4 object_color = float4(0.8, 0.8, 0.8, 1.0);
    // diffuse_texture.Sample(diffuse_sampler, input.uv).rgb

    const float3 diffuse_color = object_color.rgb * diffuse_coef * light_color;
    const float3 specular_color = float3(1.f, 1.f, 1.f) * specular_coef;

    const float3 color = (diffuse_color + specular_color) * light_attenuation;
    return color;
}

[shader("pixel")]
PsOutput main(PsInput input)
{
    const float3 normal = normalize(input.frag_normal);

    const float3 light_1_term = phong_light(input.world_pos, normal, light_1.pos, light_1.color);
    const float3 light_2_term = phong_light(input.world_pos, normal, light_2.pos, light_2.color);
    const float3 light_3_term = phong_light(input.world_pos, normal, light_3.pos, light_3.color);

    PsOutput output = (PsOutput)0;
    output.color = float4(light_1_term + light_2_term + light_3_term, 1.0f);
    return output;
}
