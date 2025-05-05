struct VsInput
{
    [[vk::location(0)]]
    float3 inPos : ACC_0;

    [[vk::location(1)]]
    float3 inNormal : ACC_1;

    [[vk::location(2)]]
    float2 inUV0 : ACC_2;

    [[vk::location(3)]]
    float2 inUV1 : ACC_3;

    [[vk::location(4)]]
    uint4 inJoint0 : ACC_4;

    [[vk::location(5)]]
    float4 inWeight0 : ACC_5;

    [[vk::location(6)]]
    float4 inColor0 : ACC_6;
};

struct VsOutput
{
    float4 outPos : SV_POSITION;

    [[vk::location(0)]]
    float3 outWorldPos : ACC_0;

    [[vk::location(1)]]
    float3 outNormal : ACC_1;

    [[vk::location(2)]]
    float2 outUV0 : ACC_2;

    [[vk::location(3)]]
    float2 outUV1 : ACC_3;

    [[vk::location(4)]]
    float4 outColor0 : ACC_4;
};

[[vk::binding(0, 0)]]
cbuffer UBO
{
    float4x4 ubo_projection;
    float4x4 ubo_model;
    float4x4 ubo_view;
    float4x4 ubo_camPos;
}

#define MAX_NUM_JOINTS 128
[[vk::binding(0, 2)]]
cbuffer UBONode
{
    float4x4 node_matrix;
    float4x4 node_jointMatrix[MAX_NUM_JOINTS];
    uint node_jointCount;
};

[shader("vertex")]
VsOutput main(VsInput input)
{
    VsOutput output = (VsOutput)0;

    output.outColor0 = input.inColor0;

    float4 locPos;
    if (node_jointCount > 0) {
        // float4 skinMat = ;
    }

    return output;
}
