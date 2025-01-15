/// 结构体名称没有特殊限制
/// 输入 semantic 是必要的
struct VsInput
{
    [[vk::location(0)]]
    float4 Position : POSITION;

    [[vk::location(1)]]
    float3 Color : COLOR0;
};

struct VsOutput
{
    /// 这个 SV_ 表示 builtin 的变量
    float4 Pos : SV_POSITION;

    [[vk::location(0)]]
    float3 Color : COLOR0;

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
