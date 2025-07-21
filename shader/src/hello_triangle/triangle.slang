struct VsInput
{
    [[vk::location(0)]]
    float4 Position : POSITION;

    [[vk::location(1)]]
    float4 Color : COLOR0;
};

struct CoarseVertex
{
    [[vk::location(0)]]
    float4 Color : COLOR0;

    [[vk::location(1)]]
    float2 UV : TEXCOORD0;
};

struct VsOutput
{
    float4 Pos : SV_POSITION;

    CoarseVertex coarse_vertex : CoarseVertex;
};

[shader("vertex")]
VsOutput vsmain(VsInput input, uint VertexIndex: SV_VertexID)
{
    VsOutput output = (VsOutput)0;
    output.coarse_vertex.Color = input.Color;
    output.coarse_vertex.UV = input.Position.xy * 0.5 + 0.5;
    output.Pos = input.Position;
    return output;
}

struct PS_OUTPUT
{
    [[vk::location(0)]]
    float4 Color : SV_TARGET0;
};

[shader("pixel")]
PS_OUTPUT psmain(CoarseVertex input)
{
    PS_OUTPUT output = (PS_OUTPUT)0;
    output.Color = input.Color;
    return output;
}
