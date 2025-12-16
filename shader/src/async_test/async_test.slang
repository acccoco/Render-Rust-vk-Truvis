#include "bindless.slangi"
#include "vertex_attribute.slangi"

struct PushConstants
{
    uint texture_id;
};

[[vk::push_constant]]
PushConstants pc;

[shader("vertex")]
soa::VsOutput vsmain(soa::VsInput input)
{
    soa::VsOutput output;
    output.Pos = float4(input.Position, 1.0);
    output.coarse_vertex.Normal = input.Normal;
    output.coarse_vertex.Tangent = input.Tangent;
    output.coarse_vertex.UV = input.UV;
    return output;
}

[shader("fragment")]
float4 psmain(soa::VsOutput input)
    : SV_Target
{
    uint idx = pc.texture_id;
    // Use NonUniformResourceIndex to handle divergent indexing
    return bindless_textures[NonUniformResourceIndex(idx)].Sample(input.coarse_vertex.UV);
}
