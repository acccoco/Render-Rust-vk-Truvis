#include "../include/scene.slangi"
#include "phong.slangi"

struct VsInput
{
    [[vk::location(0)]]
    float3 pos : LOCAL_POS;

    [[vk::location(1)]]
    float3 normal : NORMAL;

    [[vk::location(2)]]
    float2 uv : UV;
};

struct VsOutput
{
    float4 pos : SV_POSITION;

    CoarseVertex coarse_vertex : CoarseVertex;
};

[[vk::push_constant]]
DrawData draw_data;

[shader("vertex")]
VsOutput main(VsInput input)
{
    const SubMesh mesh = draw_data.frame_data.ins_data.instances[draw_data.instance_id];

    VsOutput output = (VsOutput)0;

    const float4x4 mvp = mul(draw_data.frame_data.projection, mul(draw_data.frame_data.view, mesh.model));
    output.pos = mul(mvp, float4(input.pos, 1.0));
    output.coarse_vertex.world_pos = mul(mesh.model, float4(input.pos, 1.0)).xyz;
    output.coarse_vertex.uv = input.uv;
    output.coarse_vertex.frag_normal = mul(mesh.inv_model, float4(input.normal, 0.0)).xyz;

    return output;
}
