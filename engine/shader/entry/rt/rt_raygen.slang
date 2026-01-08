/// @file rt_raygen.slang
/// @brief 光线生成着色器
///
/// 实现路径追踪的主循环，支持 SER (Shader Execution Reordering) 优化

#include "./payload.slangi"

#include "lib/bindless_op.slangi"
#include "lib/sample/sample.slangi"

// 开启后，虽然 active threads 的 coherence 指标从 50% 上升到了 70%
// 但是由于现阶段的实现开销主要在 TLAS 求交上面，而不是 closest shader 上面
// 因此整体性能并没有提升，反而略有下降
#define USE_HIT_OBJECT_SER 1

/// 计算 SER coherence hint，用于光线重排序优化
/// @param instance_id 实例 ID
/// @param geometry_id 几何体 ID
/// @param mat 材质指针
/// @return coherence hint 值（0-4）
uint computeCoherenceHint(uint instance_id, uint geometry_id, PBRMaterial* mat)
{
    const static uint MAT_TYPE_SECTION_PLANE = 0; // emissive.x > 100
    const static uint MAT_TYPE_EMISSIVE = 1;      // emissive.x > 0
    const static uint MAT_TYPE_DIFFUSE = 2;       // opaque=1, roughness>0.5
    const static uint MAT_TYPE_SPECULAR = 3;      // opaque=1, roughness<0.5
    const static uint MAT_TYPE_TRANSPARENT = 4;   // opaque<1

    uint material_type = MAT_TYPE_DIFFUSE;

    // 判断材质类型
    if (mat.emissive.x > 100.f)
    {
        material_type = MAT_TYPE_SECTION_PLANE;
    }
    else if (mat.emissive.x > 0.f)
    {
        material_type = MAT_TYPE_EMISSIVE;
    }
    else if (mat.opaque < 1.f)
    {
        material_type = MAT_TYPE_TRANSPARENT;
    }
    else if (mat.roughness < 0.5f)
    {
        material_type = MAT_TYPE_SPECULAR;
    }
    else
    {
        material_type = MAT_TYPE_DIFFUSE;
    }

    return material_type;
}

/// 生成相机光线
/// @param thread_id 当前线程 ID
/// @param subpixel_jitter 子像素抖动偏移
/// @return 光线描述
RayDesc generateCameraRay(uint2 thread_id, float2 subpixel_jitter)
{
    const float2 pixel_center = float2(thread_id) + subpixel_jitter;
    const float2 in_uv = pixel_center / float2(DispatchRaysDimensions().xy);
    const float4 target_in_view = mul(per_frame_data.inv_projection, float4(in_uv.x * 2.0 - 1.0, 1.0 - in_uv.y * 2.0, 1.0, 1.0));
    const float4 direction = mul(per_frame_data.inv_view, float4(normalize(target_in_view.xyz), 0.0));

    RayDesc ray;
    ray.Origin = per_frame_data.camera_pos;
    ray.Direction = direction.xyz;
    ray.TMin = 0.001f;
    ray.TMax = 10000.0f;
    return ray;
}

/// 初始化光线载荷
/// @param random_seed 随机种子
/// @return 初始化的载荷
HitPayload initPayload(uint random_seed)
{
    HitPayload payload = {};
    payload.radiance = float3(0.0f, 0.0f, 0.0f);
    payload.weight = float3(1.0f, 1.0f, 1.0f);
    payload.done = false;
    payload.random_seed = random_seed;
    payload.section_stack = 0;
    payload.object_stack = 0;
    return payload;
}

[shader("raygeneration")]
void main_ray_gen()
{
    const uint accum_samples = per_frame_data.accum_frames * push_const.spp + push_const.spp_idx;
    if (accum_samples >= max_accum_samples)
    {
        // 达到最大累积帧数，直接返回
        return;
    }

    uint2 thread_id = DispatchRaysIndex().xy;

    // 子像素抖动：0.375 来自于 Blender 的 Cycles
    uint random_seed = Random::tea(thread_id.y * DispatchRaysDimensions().x + thread_id.x, accum_samples);
    const float2 subpixel_jitter = (accum_samples == 0 || push_const.channel != 0) ? float2(0.5f) : float2(0.5) + 0.375 * Random::rand_gaussian(random_seed);

    RayDesc ray = generateCameraRay(thread_id, subpixel_jitter);
    HitPayload payload = initPayload(random_seed);

    // 路径追踪主循环
    // E 直接光照
    // w = BRDF * cos(theta) / pdf
    // Lo = w_0 * Li_0 + E_0
    // Li_0 = w_1 * Li_1 + E_1
    // Lo = E_0 + w_0 * E_1 + w_0 * w_1 * E_2 + ...
    //      + w_0 * ... * w_n * Li_n
    float3 color = float3(0.0f, 0.0f, 0.0f);
    float3 weight = float3(1.f, 1.f, 1.f);

#if USE_HIT_OBJECT_SER
    // 使用 Hit Object API 实现 Shader Execution Reordering
    HitObject hit_obj;

    for (uint depth = 0; depth < max_depth && !payload.done; ++depth)
    {
        hit_obj = HitObject::TraceRay(
            rt::rt_tlas, RAY_FLAG_NONE, 0xFF,
            0, // sbt record offset
            0, // sbt record stride
            0, // sky miss shader
            ray, payload
        );

        uint coherence_hint = 0;
        if (hit_obj.IsHit())
        {
            uint instance_id = hit_obj.GetInstanceIndex();
            uint geometry_id = hit_obj.GetGeometryIndex();
            PBRMaterial* mat = gpu_scene.get_material(instance_id, geometry_id);
            coherence_hint = computeCoherenceHint(instance_id, geometry_id, mat);
        }

        // 参考
        // https://www.khronos.org/blog/boosting-ray-tracing-performance-with-shader-execution-reordering-introducing-vk-ext-ray-tracing-invocation-reorder
        // hint 使用的 bits 越少越好
        // reorder 的原则：首先是 shader，然后是 hint
        ReorderThread(coherence_hint, 4);

        HitObject::Invoke(rt::rt_tlas, hit_obj, payload);

        color += payload.radiance * weight;
        weight *= payload.weight;
        ray.Origin = payload.ray_origin;
        ray.Direction = payload.ray_dir;
    }
#else
    // 标准路径：使用传统 TraceRay
    for (uint depth = 0; depth < max_depth && !payload.done; ++depth)
    {
        TraceRay(rt::rt_tlas, RAY_FLAG_NONE, 0xFF,
                 0, // sbt record offset
                 0, // sbt record stride
                 0, // sky miss shader
                 ray, payload);

        color += payload.radiance * weight;
        weight *= payload.weight;
        ray.Origin = payload.ray_origin;
        ray.Direction = payload.ray_dir;
    }
#endif

    // 累积帧混合
    if (accum_samples > 0 && push_const.channel == 0)
    {
        float a = 1.f / float(accum_samples + 1);
        const float3 old_color = rt::rt_color.Load(thread_id).xyz;
        rt::rt_color.Store(thread_id, float4(lerp(old_color, color, a), 1.f));
    }
    else
    {
        rt::rt_color.Store(thread_id, float4(color, 1.f));
    }
}
