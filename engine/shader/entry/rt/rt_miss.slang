/// @file rt_miss.slang
/// @brief Miss 着色器（重构版）
///
/// 处理光线未命中任何几何体时的情况（天空盒、阴影测试）

#include "./payload.slangi"

#include "lib/bindless_op.slangi"
#include "lib/env_map.slangi"

// ============================================================================
// Sky Miss Shader
// ============================================================================

/// 天空 miss 着色器
/// 当光线未命中任何几何体时，返回天空信息
[shader("miss")]
void sky_miss(inout HitPayload payload)
{
    const float2 uv = dir_to_env_uv(WorldRayDirection());
    const float3 sky_color = float3(bindless_srv::sample_level(gpu_scene.sky, uv, gpu_scene.sky_sampler_type, 0.0).xyz) * 8.f;

    payload.hit = false;

    // 填充天空信息作为"虚拟命中"
    payload.info.position = float3(0.f);
    payload.info.normal = -WorldRayDirection();
    payload.info.origin_normal = -WorldRayDirection();
    payload.info.uv = uv;

    payload.info.base_color = sky_color;
    payload.info.metallic = 0.f;
    payload.info.roughness = 1.f;
    payload.info.opaque = 1.f;
    payload.info.ior = 1.f;

    payload.info.emissive = sky_color;
    payload.info.material_type = MaterialType::SKY;
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
