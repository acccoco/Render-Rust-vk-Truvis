struct PsInput
{
    [[vk::location(0)]]
    float3 world_pos : ACC_0;

    [[vk::location(1)]]
    float3 frag_normal : ACC_1;

    [[vk::location(2)]]
    float2 uv : ACC_2;
};

struct PsOutput
{
    [[vk::location(0)]]
    float4 color : SV_TARGET0;
};

struct PushConstants
{
    float3 camera_pos;
    float3 camera_dir;
    uint frame_id;
    float delta_time_ms;
    float2 mouse;
    float2 resolution;
    float time;
    float frame_rate;
};

[[vk::push_constant]]
PushConstants push_constants;

[[vk::binding(0, 0)]]
cbuffer SceneUBO
{
    float3 light_pos;
    float3 light_color;
    float4x4 projection;
    float4x4 view;
};

[[vk::binding(0, 0)]]
cbuffer MaterialUBO
{
    float4 color;
};

// [[vk::binding(0, 1)]]
// [[vk::combinedImageSampler]]
// Texture2D<float4> diffuse_texture;

// [[vk::binding(0, 1)]]
// [[vk::combinedImageSampler]]
// SamplerState diffuse_sampler;

PsOutput main(PsInput input)
{
    const float3 light_dir = normalize(input.world_pos - light_pos);
    const float3 view_dir = normalize(input.world_pos - push_constants.camera_pos);
    const float3 normal = normalize(input.frag_normal);
    const float3 reflect_dir = normalize(reflect(light_dir, normal));
    const float3 halfway = -normalize(light_dir + view_dir);

    const float diffuse_coef = max(0.0, dot(-light_dir, normal));

    const float specular_coef = pow(max(0.0, dot(normal, halfway)), 128.0);

    const float const_term = 1.0f;
    const float linear_term = 0.09f;
    const float quadratic_term = 0.032f;
    const float distance = length(light_pos - input.world_pos);
    const float light_attenuation = 1.0 / (const_term + linear_term * distance + quadratic_term * distance * distance);

    const float4 color = color;
    // diffuse_texture.Sample(diffuse_sampler, input.uv).rgb
    const float3 diffuse_color = color.rgb * diffuse_coef * light_color;
    const float3 specular_color = float3(1.f, 1.f, 1.f) * specular_coef;

    PsOutput output = (PsOutput)0;
    output.color = float4((diffuse_color + specular_color) * light_attenuation, 1.0f);
    output.color = float4(0.8, 0.8, 0.8, 1.0);
    return output;
}
