struct VsInput
{
    [[vk::location(0)]]
    float4 Position : POSITION;

    [[vk::location(1)]]
    float4 Color : COLOR0;
};

struct VsOutput
{
    float4 Pos : SV_POSITION;

    [[vk::location(0)]]
    float4 Color : COLOR0;

    [[vk::location(1)]]
    float2 UV : TEXCOORD0;
};

VsOutput main(VsInput input, uint VertexIndex: SV_VertexID)
{
    VsOutput output = (VsOutput)0;
    output.Color = input.Color;
    output.UV = input.Position.xy * 0.5 + 0.5;
    output.Pos = input.Position;
    return output;
}
