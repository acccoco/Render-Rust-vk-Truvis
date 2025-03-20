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

#define ParamBlock_Font(name, set)                 \
    [[vk::combinedImageSampler]]   \
    [[vk::binding(0, set)]]          \
    Texture2D<float4> name##Texture; \
                                   \
    [[vk::combinedImageSampler]]   \
    [[vk::binding(0, set)]]          \
    SamplerState name##Sampler;

ParamBlock_Font(font, 0)

PsOutput main(PsInput input)
{
    PsOutput output = (PsOutput)0;

    output.color = input.color * fontTexture.Sample(fontSampler, input.uv);
    return output;
}
