struct PushConstants
{
    float4x4 ortho;
};

[[vk::push_constant]]
PushConstants matrices;

struct VsInput
{
    [[vk::location(0)]]
    float2 pos : I0;

    [[vk::location(1)]]
    float2 uv : I1;

    [[vk::location(2)]]
    float4 color : I2;
};

struct VsOutput
{
    float4 pos : SV_Position;

    [[vk::location(0)]]
    float4 color : O0;

    [[vk::location(1)]]
    float2 uv : O1;
};

[shader("vertex")]
VsOutput vsmain(VsInput input)
{
    VsOutput output = (VsOutput)0;
    output.color = input.color;
    output.uv = input.uv;
    output.pos = mul(matrices.ortho, float4(input.pos, 0.0, 1.0));
    return output;
}

struct PsInput
{
    [[vk::location(0)]]
    float4 color : I0;

    [[vk::location(1)]]
    float2 uv : I1;
};

struct PsOutput
{
    [[vk::location(0)]]
    float4 color : SV_TARGET0;
};

[[vk::binding(0, 0)]]
Sampler2D<float4> fontTexture;

[shader("pixel")]
PsOutput psmain(PsInput input)
{
    PsOutput output = (PsOutput)0;

    output.color = input.color * fontTexture.Sample(input.uv);
    return output;
}
