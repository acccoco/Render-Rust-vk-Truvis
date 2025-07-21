#include "pass/imgui.slangi"

[[vk::push_constant]]
imgui::PushConstant push_constant;

struct CoarseVertex
{

    [[vk::location(0)]]
    float4 color : COLOR;

    [[vk::location(1)]]
    float2 uv : UV;
};

struct VsInput
{
    [[vk::location(0)]]
    float2 pos : POS;

    [[vk::location(1)]]
    float2 uv : UV;

    [[vk::location(2)]]
    float4 color : COLOR;
};

struct VsOutput
{
    float4 pos : SV_Position;

    CoarseVertex coarse_vertex : COARSE_VERTEX;
};

[shader("vertex")]
VsOutput vsmain(VsInput input)
{
    VsOutput output = (VsOutput)0;
    output.coarse_vertex.color = input.color;
    output.coarse_vertex.uv = input.uv;
    output.pos = mul(push_constant.ortho, float4(input.pos, 0.0, 1.0));
    return output;
}

struct PsInput
{
    CoarseVertex coarse_vertex : COARSE_VERTEX;
};

struct PsOutput
{
    [[vk::location(0)]]
    float4 color : SV_TARGET0;
};

[shader("pixel")]
PsOutput psmain(PsInput input)
{
    PsOutput output = (PsOutput)0;

    output.color = input.coarse_vertex.color * push_constant.texture.sample(input.coarse_vertex.uv);
    return output;
}
