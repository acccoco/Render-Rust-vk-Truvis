struct PsInput
{
    [[vk::location(0)]]
    float4 color : ARRAY;

    [[vk::location(1)]]
    float2 uv : TEXCOORD0;
};

struct PsOutput
{
    [[vk::location(0)]]
    float4 color : SV_TARGET0;
};

[[vk::combinedImageSampler]]
[[vk::binding(0, 0)]]
Texture2D<float4> fontTexture;

[[vk::combinedImageSampler]]
[[vk::binding(0, 0)]]
SamplerState fontSampler;

PsOutput main(PsInput input)
{
    PsOutput output = (PsOutput)0;

    output.color = input.color * fontTexture.Sample(fontSampler, input.uv);
    return output;
}
