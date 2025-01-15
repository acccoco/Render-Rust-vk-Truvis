struct VSInput
{
    [[vk::location(0)]]
    float4 Position : POSITION0;

    [[vk::location(1)]]
    float3 Color : COLOR0;
};

struct VSOutput
{
    float4 Pos : SV_POSITION;

    [[vk::location(0)]]
    float3 Color : COLOR0;

    [[vk::location(1)]]
    float2 UV : TEXCOORD0;
};

VSOutput main(VSInput input, uint VertexIndex: SV_VertexID)
{

    VSOutput output = (VSOutput)0;
    output.Color = input.Color;
    output.UV = input.Position.xy * 0.5 + 0.5;
    output.Pos = input.Position;
    return output;
}
