/// @file rt_miss.slang
/// @brief Miss 着色器
///
/// 处理光线未命中任何几何体时的情况（天空盒、阴影测试）

#include "./payload.slangi"

#include "lib/bindless_op.slangi"
#include "lib/env_map.slangi"

// ============================================================================
// Sky Miss Shader
// ============================================================================

/// 天空 miss 着色器
/// 当光线未命中任何几何体时，采样环境贴图作为天空颜色
[shader("miss")]
void sky_miss(inout HitPayload payload)
{
    const float2 uv = dir_to_env_uv(WorldRayDirection());
    payload.emission = float3(bindless_srv::sample_level(gpu_scene.sky, uv, gpu_scene.sky_sampler_type, 0.0).xyz) * 8.f;
    payload.done = true;
}

// ============================================================================
// Shadow Miss Shader
// ============================================================================

/// 阴影 miss 着色器
/// 用于阴影光线测试，如果未命中任何物体则表示该点可见光源
///
/// 阴影光线的 flag:
/// const uint ray_flags =
///     RAY_FLAG_FORCE_OPAQUE                      // 强制不透明
///     | RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH // 接受第一个命中并结束搜索
///     | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER         // 跳过 closest hit shader
///     ;
[shader("miss")]
void shadow_miss(inout ShadowMissPayload payload)
{
    payload.miss = true;
}
