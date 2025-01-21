struct VsInput
{
    [[vk::location(0)]]
    float3 pos : ACC_0;

    [[vk::location(1)]]
    float3 normal : ACC_1;

    [[vk::location(2)]]
    float2 uv : ACC_2;
};

struct VsOutput
{
    float4 pos : SV_POSITION;

    [[vk::location(0)]]
    float3 world_pos : ACC_0;

    [[vk::location(1)]]
    float3 frag_normal : ACC_1;

    [[vk::location(2)]]
    float2 uv : ACC_2;
};

[[vk::binding(0, 0)]]
cbuffer SceneUBO
{
    float3 light_pos;
    float3 light_color;
    float4x4 projection;
    float4x4 view;
};

[[vk::binding(0, 1)]]
cbuffer MeshUBO
{
    float4x4 model;
    float4x4 trans_inv_model;
};

VsOutput main(VsInput input)
{
    VsOutput output = (VsOutput)0;

    const float4x4 mvp = mul(projection, mul(view, model));
    output.pos = mul(mvp, float4(input.pos, 1.0));
    output.world_pos = mul(model, float4(input.pos, 1.0)).xyz;
    output.uv = input.uv;
    output.frag_normal = mul(trans_inv_model, float4(input.normal, 0.0)).xyz;

    return output;
}
