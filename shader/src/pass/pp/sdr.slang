#include "pass/pp/sdr.slangi"
#include "sample/random.slangi"

[push_constant]
sdr::PushConstant g_params;

[shader("compute")]
[numthreads(sdr::SHADER_X, sdr::SHADER_Y, 1)]
void main(uint3 dispatchThreadID: SV_DispatchThreadID)
{
    if (dispatchThreadID.x > g_params.image_size.x ||
        dispatchThreadID.y > g_params.image_size.y)
    {
        return; // Out of bounds
    }

    const float4 hdr_color = g_params.src_image.load(dispatchThreadID.xy);

    // TODO 引入 accum frame 的参数，增加随机性
    // 引入噪声消除色阶
    uint random_seed = Random::tea(dispatchThreadID.y, dispatchThreadID.x);
    const float delta = Random::rnd(random_seed);
    const float3 sdr_color = hdr_color.rgb / (0.5 + hdr_color.rgb) + delta / 255.f;
    // const float3 sdr_color = hdr_color.rgb;

    g_params.dst_image.store(
        dispatchThreadID.xy,
        float4(sdr_color, hdr_color.a));
}
