#include "blit.slangi"

[push_constant]
blit::PushConstant gParams;

[shader("compute")]
[numthreads(blit::SHADER_X, blit::SHADER_Y, 1)]
void main(uint3 dispatchThreadID: SV_DispatchThreadID)
{
    if (dispatchThreadID.x > gParams.src_image_size.x ||
        dispatchThreadID.y > gParams.src_image_size.y)
    {
        return; // Out of bounds
    }

    gParams.dst_image.store(
        dispatchThreadID.xy + gParams.offset,
        gParams.src_image.load(dispatchThreadID.xy));
}
