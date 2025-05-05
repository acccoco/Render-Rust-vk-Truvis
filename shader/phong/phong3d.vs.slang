#include "phong.slangi"

struct VsInput
{
    [[vk::location(0)]]
    float3 pos : LOCAL_POS;

    [[vk::location(1)]]
    float3 normal : NORMAL;

    [[vk::location(2)]]
    float3 tangent : TANGENT;

    [[vk::location(3)]]
    float3 bitangent : BITANGENT;

    [[vk::location(4)]]
    float2 uv : UV;
};

struct CoarseVertex
{
    [[vk::location(0)]]
    float3 world_pos : WORLD_POS;

    [[vk::location(1)]]
    float3 frag_normal : FRAG_NORMAL;

    [[vk::location(2)]]
    float2 uv : UV;
};

struct VsOutput
{
    float4 pos : SV_POSITION;

    CoarseVertex coarse_vertex : CoarseVertex;
};

[[vk::binding(0, 0)]]
cbuffer SceneUBO
{
    float4x4 projection;
    float4x4 view;

    Light light_1;
    Light light_2;
    Light light_3;
};

[[vk::binding(0, 1)]]
cbuffer MeshUBO
{
    float4x4 model;
    float4x4 trans_inv_model;
};

[[vk::push_constant]]
PushConstants push_constants;

[shader("vertex")]
VsOutput main(VsInput input)
{
    VsOutput output = (VsOutput)0;

    // SceneData scene = vk::RawBufferLoad<SceneData>(push_constants.scene_buffer_addr, 4);
    SceneData scene = *push_constants.scene_buffer_ptr;

    const float4x4 mvp = mul(projection, mul(scene.view, push_constants.model));
    output.pos = mul(mvp, float4(input.pos, 1.0));
    output.coarse_vertex.world_pos = mul(push_constants.model, float4(input.pos, 1.0)).xyz;
    output.coarse_vertex.uv = input.uv;
    output.coarse_vertex.frag_normal = mul(push_constants.inv_model, float4(input.normal, 0.0)).xyz;

    return output;
}
