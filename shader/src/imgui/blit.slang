#include "pass/blit.slangi"

[push_constant]
blit::PushConstant g_params;

[shader("compute")]
[numthreads(blit::SHADER_X, blit::SHADER_Y, 1)]
void main(uint3 dispatchThreadID: SV_DispatchThreadID)
{
    if (dispatchThreadID.x > g_params.src_image_size.x ||
        dispatchThreadID.y > g_params.src_image_size.y)
    {
        return; // Out of bounds
    }

    g_params.dst_image.store(
        dispatchThreadID.xy + g_params.offset,
        // g_params.src_image.load(dispatchThreadID.xy)
        float4(1.0, 0.0, 0.0, 1.0)
    );
}
