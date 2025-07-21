struct PushConstant
{
    float time;
    float delta_time;
    int frame;
    float frame_rate;
    float2 resolution;
};

[[vk::push_constant]]
PushConstant pc;

struct VsInput
{
    [[vk::location(0)]]
    float4 pos : ACC_0;

    [[vk::location(1)]]
    float4 color : ACC_1;
};

struct VsOutput
{
    float4 pos : SV_Position;

    [[vk::location(0)]]
    float2 fragCoord : ACC_0;
};

VsOutput main(VsInput input)
{
    VsOutput output = (VsOutput)0;

    // NDC 坐标系是 left-top(-1, -1), right-bottom(1, 1)
    // shadertoy 坐标系是 left-bottom(0, 0), right-top(1, 1)
    output.fragCoord = (input.pos.xy * float2(0.5, -0.5) + 0.5) * pc.resolution;
    output.pos = input.pos;

    return output;
}
