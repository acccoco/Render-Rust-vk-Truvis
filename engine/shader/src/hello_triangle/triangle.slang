#include "vertex_attribute.slangi"

[shader("vertex")]
soa::VsOutput vsmain(soa::VsInput input, uint VertexIndex: SV_VertexID)
{
    soa::VsOutput output = (soa::VsOutput)0;
    output.coarse_vertex.UV = input.UV;
    output.coarse_vertex.Normal = input.Normal;
    output.coarse_vertex.Tangent = input.Tangent;
    output.Pos = float4(input.Position, 1.0);
    return output;
}

[shader("pixel")]
soa::PS_OUTPUT psmain(soa::CoarseVertex input)
{
    soa::PS_OUTPUT output = (soa::PS_OUTPUT)0;
    output.Color = float4(
        float3(input.UV, 0.0) * 0.7 + input.Normal * 0.15 + input.Tangent * 0.15,
        1.0
    );
    return output;
}
