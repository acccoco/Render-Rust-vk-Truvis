struct PushConstants
{
    float4x4 ortho;
};

[[vk::push_constant]]
PushConstants matrices;

struct VsInput
{
    [[vk::location(0)]]
    float2 pos : POSITION;

    [[vk::location(1)]]
    float2 uv : TEXCOORD0;

    [[vk::location(2)]]
    float4 color : COLOR0;
};

struct VsOutput
{
    float4 pos : SV_Position;

    [[vk::location(0)]]
    float4 color : COLOR0;

    [[vk::location(1)]]
    float2 uv : TEXCOORD0;
};

VsOutput main(VsInput input)
{
    VsOutput output = (VsOutput)0;
    output.color = input.color;
    output.uv = input.uv;
    output.pos = mul(matrices.ortho, float4(input.pos, 0.0, 1.0));
    return output;
}
