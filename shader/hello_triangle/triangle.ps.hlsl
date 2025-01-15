/// 结构体名称并没有强制规定
/// sementics 是必要的，但是 spv 并不会使用吧
struct PS_INPUT
{
    [[vk::location(0)]]
    float4 Color : COLOR0;

    [[vk::location(1)]]
    float2 UV : TEXCOORD0;
};

struct PS_OUTPUT
{
    [[vk::location(0)]]
    float4 Color : SV_TARGET0;
};

PS_OUTPUT main(PS_INPUT input)
{
    PS_OUTPUT output = (PS_OUTPUT)0;
    output.Color = float4(input.UV, 0.0, 1.0);
    return output;
}