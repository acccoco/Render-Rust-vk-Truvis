/// @file rt_raygen.slang
/// @brief 光线生成着色器（重构版）
///
/// 所有材质计算都在 ray gen 中进行：
/// - BRDF 采样和计算
/// - throughput 计算
/// - radiance 累积
/// - 俄罗斯轮盘赌终止

#include "./payload.slangi"

#include "lib/bindless_op.slangi"
#include "lib/env_map.slangi"
#include "lib/mis.slangi"
#include "lib/pbr.slangi"
#include "lib/sample/random.slangi"
#include "lib/sample/sample.slangi"

// 开启后，虽然 active threads 的 coherence 指标从 50% 上升到了 70%
// SER 主要用于提升后续新的 shader invocation 的一致性
// 当前的架构重 ray-gen，轻 hit/miss shader，SER 带来的提升有限
#define USE_HIT_OBJECT_SER 0

// ============================================================================
// 相机和初始化
// ============================================================================

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
    payload.hit = false;
    payload.random_seed = random_seed;
    payload.section_stack = 0;
    payload.object_stack = 0;
    return payload;
}

// ============================================================================
// BRDF 采样函数
// ============================================================================

/// 漫反射采样
/// @param info 命中信息
/// @param random_seed 随机种子
/// @param out_dir 输出：采样方向
/// @param out_throughput 输出：throughput = BRDF * cos / pdf
/// @param out_pdf 输出：采样 PDF
void sampleDiffuse(
    const HitInfo info,
    inout uint random_seed,
    out float3 out_dir,
    out float3 out_throughput,
    out float out_pdf
)
{
    // 半球 cos 采样
    // BRDF = diffuse_color / pi
    // pdf = cos(theta) / pi
    // throughput = BRDF * cos(theta) / pdf = diffuse_color
    const float3 local_dir = Sample::get_cos_hemisphere_sample(random_seed);
    float3 tangent, bitangent;
    Sample::create_local_coord(info.normal, tangent, bitangent);
    out_dir = normalize(local_dir.x * tangent + local_dir.y * bitangent + local_dir.z * info.normal);

    const float3 diffuse_color = (1.f - info.metallic) * info.base_color;
    out_throughput = diffuse_color;
    out_pdf = DiffuseBrdf::pdf(info.normal, out_dir);
}

/// 镜面反射采样（GGX）
/// @param info 命中信息
/// @param view_dir 观察方向（从命中点到相机）
/// @param random_seed 随机种子
/// @param out_dir 输出：采样方向
/// @param out_throughput 输出：throughput = BRDF * cos / pdf
/// @param out_pdf 输出：采样 PDF
void sampleSpecular(
    const HitInfo info,
    const float3 view_dir,
    inout uint random_seed,
    out float3 out_dir,
    out float3 out_throughput,
    out float out_pdf
)
{
    const float reflectance = 0.8f;
    const float3 f0 = lerp(0.16 * reflectance * reflectance, info.base_color, info.metallic);
    const float real_roughness = info.roughness * info.roughness;

    // 根据 NDF 采样半程向量 H
    float3 h = SpecularBrdf::ggx_sample(random_seed, real_roughness, info.normal);
    SpecularBrdf specular_brdf = SpecularBrdf(f0, real_roughness, info.normal, view_dir, h);

    out_throughput = specular_brdf.brdf_with_pdf() * specular_brdf.get_ndotl();
    out_dir = specular_brdf.light;
    out_pdf = specular_brdf.ggx_pdf();
}

/// 完美镜面反射（delta 路径）
/// @param info 命中信息
/// @param ray_dir 入射光线方向
/// @param out_dir 输出：反射方向
/// @param out_throughput 输出：throughput
void samplePerfectReflection(
    const HitInfo info,
    const float3 ray_dir,
    out float3 out_dir,
    out float3 out_throughput
)
{
    out_dir = reflect(ray_dir, info.normal);
    // 完美镜面反射的 throughput 基于菲涅尔
    const float reflectance = 0.8f;
    const float3 f0 = lerp(0.16 * reflectance * reflectance, info.base_color, info.metallic);
    const float cos_theta = saturate(dot(-ray_dir, info.normal));
    const float fresnel = pow(1.f - cos_theta, 5.f);
    out_throughput = f0 + (1.f - f0) * fresnel;
}

/// 折射采样
/// @param info 命中信息
/// @param ray_dir 入射光线方向
/// @param out_dir 输出：折射方向
/// @param out_throughput 输出：throughput
/// @return true 表示折射成功，false 表示全内反射
bool sampleRefraction(
    const HitInfo info,
    const float3 ray_dir,
    out float3 out_dir,
    out float3 out_throughput
)
{
    // 根据光线与原始法线的夹角判断是进入还是离开介质
    const float eta = dot(ray_dir, info.origin_normal) > 0.f ? info.ior : 1.f / info.ior;

    out_dir = refract(ray_dir, info.normal, eta);
    if (length(out_dir) < 1e-6f)
    {
        out_dir = float3(0.f);
        out_throughput = float3(0.f);
        // 全内反射
        return false;
    }

    out_throughput = info.base_color;
    return true;
}

// ============================================================================
// 材质处理
// ============================================================================

/// 处理剖切体逻辑
/// @param payload 光线载荷
/// @param info 命中信息
/// @param ray_dir 当前光线方向
/// @param out_origin 输出：下一条光线起点
/// @param out_dir 输出：下一条光线方向
/// @param out_emission 输出：自发光
/// @param out_done 输出：是否终止
/// @return true 表示是剖切体，已处理
bool handleSectionPlane(
    inout HitPayload payload,
    const HitInfo info,
    const float3 ray_dir,
    out float3 out_origin,
    out float3 out_dir,
    out float3 out_emission,
    out bool out_done
)
{
    out_emission = float3(0.f);
    out_done = false;

    if (info.material_type != MaterialType::SECTION)
    {
        out_origin = float3(0.f);
        out_dir = float3(0.f);
        out_emission = float3(0.f);
        out_done = false;
        return false;
    }

    payload.section_stack += dot(ray_dir, info.origin_normal) < 0.f ? 1 : -1;

    if (payload.section_stack == 0)
    {
        // 离开剖切体
        if (payload.object_stack > 0)
        {
            // 在物体内部，设为填充面
            out_done = true;
            out_emission = float3(0.8f, 0.1f, 0.8f); // 紫色
        }
        else
        {
            // 继续穿透
            out_origin = info.position + 0.0001f * ray_dir;
            out_dir = ray_dir;
        }
    }
    else
    {
        // 仍然在剖切体内部，继续穿透
        out_origin = info.position + 0.0001f * ray_dir;
        out_dir = ray_dir;
    }

    return true;
}

/// 处理剖切体内部的物体
/// @param payload 光线载荷
/// @param info 命中信息
/// @param ray_dir 当前光线方向
/// @param out_origin 输出：下一条光线起点
/// @param out_dir 输出：下一条光线方向
/// @return true 表示在剖切体内部，已处理
bool handleInsideSection(
    inout HitPayload payload,
    const HitInfo info,
    const float3 ray_dir,
    out float3 out_origin,
    out float3 out_dir
)
{
    if (payload.section_stack <= 0)
    {
        out_origin = float3(0.f);
        out_dir = float3(0.f);
        return false;
    }

    // 在剖切体内部，穿透物体
    out_origin = info.position + 0.0001f * ray_dir;
    out_dir = ray_dir;
    payload.object_stack += dot(ray_dir, info.origin_normal) < 0.f ? 1 : -1;
    return true;
}

/// 处理材质并采样下一条光线
/// @param payload 光线载荷
/// @param info 命中信息
/// @param ray_dir 当前光线方向
/// @param out_origin 输出：下一条光线起点
/// @param out_dir 输出：下一条光线方向
/// @param out_throughput 输出：throughput
/// @param out_emission 输出：自发光
/// @param out_done 输出：是否终止
/// @param out_brdf_pdf 输出：BRDF 采样 PDF（用于 MIS）
void processMaterial(
    inout HitPayload payload,
    const HitInfo info,
    const float3 ray_dir,
    out float3 out_origin,
    out float3 out_dir,
    out float3 out_throughput,
    out float3 out_emission,
    out bool out_done,
    out float out_brdf_pdf
)
{
    out_emission = float3(0.f);
    out_throughput = float3(1.f);
    out_done = false;
    out_origin = info.position;
    out_dir = ray_dir;
    out_brdf_pdf = 1.f; // Delta 路径的 PDF 设为 1（不参与 MIS）

    // 处理剖切体
    if (handleSectionPlane(payload, info, ray_dir, out_origin, out_dir, out_emission, out_done))
    {
        return;
    }

    // 处理天空/自发光
    if (info.material_type == MaterialType::SKY || info.material_type == MaterialType::EMISSIVE)
    {
        out_emission = info.emissive;
        out_done = true;
        return;
    }

    // 处理剖切体内部的物体
    if (handleInsideSection(payload, info, ray_dir, out_origin, out_dir))
    {
        return;
    }

    // 观察方向（从命中点到相机）
    const float3 view_dir = -ray_dir;

    // 根据材质类型采样
    switch (info.material_type)
    {
    case MaterialType::TRANSPARENT: {
        // 折射 vs 反射的选择
        const bool choose_refract = Random::rnd(payload.random_seed) >= info.opaque;
        if (choose_refract)
        {
            if (!sampleRefraction(info, ray_dir, out_dir, out_throughput))
            {
                // 全内反射
                samplePerfectReflection(info, ray_dir, out_dir, out_throughput);
            }
            out_origin = info.position + 0.0001f * out_dir;
        }
        else
        {
            // 反射
            samplePerfectReflection(info, ray_dir, out_dir, out_throughput);
            out_origin = info.position + 0.0001f * out_dir;
        }
        // Delta 路径，out_brdf_pdf 保持为 1
    }
    break;

    case MaterialType::SPECULAR: {
        // Delta 镜面反射
        samplePerfectReflection(info, ray_dir, out_dir, out_throughput);
        out_origin = info.position + 0.0001f * out_dir;
        // Delta 路径，out_brdf_pdf 保持为 1
    }
    break;

    case MaterialType::DIFFUSE:
    default: {
        // 漫反射 + 镜面反射的混合
        const float prob_diffuse = info.roughness;
        const bool choose_diffuse = Random::rnd(payload.random_seed) < prob_diffuse;
        float sample_pdf;

        if (choose_diffuse)
        {
            sampleDiffuse(info, payload.random_seed, out_dir, out_throughput, sample_pdf);
            out_throughput /= prob_diffuse;           // 重要性采样补偿
            out_brdf_pdf = sample_pdf * prob_diffuse; // 混合 PDF
        }
        else
        {
            sampleSpecular(info, view_dir, payload.random_seed, out_dir, out_throughput, sample_pdf);
            out_throughput /= (1.f - prob_diffuse);           // 重要性采样补偿
            out_brdf_pdf = sample_pdf * (1.f - prob_diffuse); // 混合 PDF
        }
        out_origin = info.position + 0.0001f * out_dir;
    }
    break;
    }
}

// ============================================================================
// BRDF 评估函数（用于 NEE）
// ============================================================================

/// 计算给定方向的 BRDF PDF（用于 MIS）
/// @param info 命中信息
/// @param view_dir 观察方向（从命中点到相机）
/// @param light_dir 光照方向（从命中点到光源）
/// @return 混合 BRDF 的 PDF
float evalBrdfPdf(const HitInfo info, const float3 view_dir, const float3 light_dir)
{
    // Delta 材质没有 PDF（不参与 NEE）
    if (info.is_delta_path())
    {
        return 0.f;
    }

    const float prob_diffuse = info.roughness;

    // 漫反射 PDF
    const float diffuse_pdf = DiffuseBrdf::pdf(info.normal, light_dir);

    // 镜面反射 PDF
    const float3 h = normalize(view_dir + light_dir);
    const float reflectance = 0.8f;
    const float3 f0 = lerp(0.16 * reflectance * reflectance, info.base_color, info.metallic);
    const float real_roughness = info.roughness * info.roughness;
    SpecularBrdf specular_brdf = SpecularBrdf(f0, real_roughness, info.normal, view_dir, h);
    const float specular_pdf = specular_brdf.ggx_pdf();

    // 混合 PDF
    return prob_diffuse * diffuse_pdf + (1.f - prob_diffuse) * specular_pdf;
}

/// 评估 BRDF 值（用于 NEE 直接光照计算）
/// @param info 命中信息
/// @param view_dir 观察方向（从命中点到相机）
/// @param light_dir 光照方向（从命中点到光源）
/// @return BRDF * cos(theta)
float3 evalBrdf(const HitInfo info, const float3 view_dir, const float3 light_dir)
{
    const float ndotl = max(dot(info.normal, light_dir), 0.f);
    if (ndotl <= 0.f)
    {
        return float3(0.f);
    }

    // Diffuse BRDF
    const float3 diffuse_color = (1.f - info.metallic) * info.base_color;
    const float3 diffuse = diffuse_color / M_PI;

    // Specular BRDF
    const float3 h = normalize(view_dir + light_dir);
    const float reflectance = 0.8f;
    const float3 f0 = lerp(0.16 * reflectance * reflectance, info.base_color, info.metallic);
    const float real_roughness = info.roughness * info.roughness;
    SpecularBrdf specular_brdf = SpecularBrdf(f0, real_roughness, info.normal, view_dir, h);
    const float3 specular = specular_brdf.brdf();

    return (diffuse + specular) * ndotl;
}

// ============================================================================
// 调试通道
// ============================================================================

/// 处理调试通道（首次
/// @param info 命中信息
/// @param channel 调试通道号
/// @param out_color 输出颜色
/// @return true 表示是调试模式
/// @note 通道 0: 正常渲染
///       通道 1: 法线可视化
///       通道 2: 基础颜色
///       通道 4/5/6: 需要完整路径追踪，在此函数中不处理
bool handleDebugChannel(const HitInfo info, uint channel, out float3 out_color)
{
    out_color = float3(0.f);

    if (channel == 0)
    {
        return false;
    }

    // 通道 1：法线可视化
    if (channel == 1)
    {
        out_color = info.normal * 0.5f + 0.5f;
        return true;
    }

    // 通道 2：基础颜色
    if (channel == 2)
    {
        out_color = info.base_color;
        return true;
    }

    return false;
}

// ============================================================================
// SER (Shader Execution Reordering)
// ============================================================================

/// 计算 SER coherence hint
uint computeCoherenceHint(const HitInfo info)
{
    return uint(info.material_type);
}

/// 计算 luminance
float luminance(float3 color)
{
    return dot(color, float3(0.2126f, 0.7152f, 0.0722f));
}

// trace shadow ray inline
bool shadow_ray(RaytracingAccelerationStructure tlas, const RayDesc ray)
{
    RayQuery<RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER> rq;
    rq.TraceRayInline(
        tlas,
        RAY_FLAG_NONE, // 上面已经限制了范围
        0xFF,          // cull mask
        ray
    );
    while (rq.Proceed())
    {
        if (rq.CommittedStatus() != COMMITTED_NOTHING)
        {
            return false; // hit
        }
    }
    return true; // miss
}

// ============================================================================
// 主函数
// ============================================================================

[shader("raygeneration")]
void main_ray_gen()
{
    const uint accum_samples = per_frame_data.accum_frames * push_const.spp + push_const.spp_idx;
    if (accum_samples >= max_accum_samples)
    {
        return;
    }

    uint2 thread_id = DispatchRaysIndex().xy;

    // 子像素抖动
    uint random_seed = Random::tea(thread_id.y * DispatchRaysDimensions().x + thread_id.x, accum_samples);
    const float2 subpixel_jitter = (accum_samples == 0 || push_const.channel != 0) ? float2(0.5f) : float2(0.5f) + 0.375f * Random::rand_gaussian(random_seed);

    RayDesc ray = generateCameraRay(thread_id, subpixel_jitter);
    HitPayload payload = initPayload(random_seed);

    // 路径追踪主循环
    // Lo = E_0 + w_0 * E_1 + w_0 * w_1 * E_2 + ... + w_0 * ... * w_n * Li_n
    float3 radiance = float3(0.f);
    float3 throughput = float3(1.f);
    float prev_brdf_pdf = 1.f;
    bool has_diffuse = false;  // 跟踪路径上是否遇到了 diffuse
    bool prev_is_delta = true; // 上一次是否为 delta 路径（首次弹射视为 delta）

    // 调试通道 4/5/6：分离的 radiance 累积
    float3 nee_radiance = float3(0.f);      // 通道 4：NEE 命中 HDRI 的贡献
    float3 emissive_radiance = float3(0.f); // 通道 5：自发光物体的贡献
    float3 hdri_radiance = float3(0.f);     // 通道 6：最终命中 HDRI 的贡献

    for (uint depth = 0; depth < max_depth; ++depth)
    {
        TraceRay(
            rt::rt_tlas,
            RAY_FLAG_NONE,
            0xFF, // cull mask
            0,    // ray type
            0,    // hit group index
            0,    // miss shader index
            ray, payload
        );

        // 调试通道
        float3 debug_color;
        if (handleDebugChannel(payload.info, push_const.channel, debug_color))
        {
            radiance = debug_color;
            break;
        }

        // ================================================================
        // 处理 miss（命中 HDRI）
        // ================================================================
        if (!payload.hit)
        {
            // sky_color 已在 miss shader 中采样并存储于 payload.info.emissive
            const float3 sky_color = payload.info.emissive;

            if (prev_is_delta || depth == 0)
            {
                // Delta 路径或首次弹射：直接累加，不使用 MIS
                const float3 contrib = sky_color * throughput;
                radiance += contrib;
                hdri_radiance += contrib; // 调试通道 6
            }
            else
            {
                // 非 delta 路径：使用 MIS 加权
                const float env_pdf = envMapUniformPdf();
                const float mis_weight = MIS::powerHeuristic(prev_brdf_pdf, env_pdf);
                const float3 contrib = sky_color * throughput * mis_weight;
                radiance += contrib;
                hdri_radiance += contrib; // 调试通道 6
            }
            break;
        }

        // ================================================================
        // 处理自发光物体（不使用 MIS）
        // ================================================================
        if (payload.info.material_type == MaterialType::EMISSIVE)
        {
            // 自发光物体：直接累加，不使用 MIS 权重
            const float3 contrib = payload.info.emissive * throughput;
            radiance += contrib;
            emissive_radiance += contrib; // 调试通道 5
            break;
        }

        // ================================================================
        // NEE: 下一事件估计（仅对非 delta 材质）
        // ================================================================
        if (!payload.info.is_delta_path())
        {
            // 在 HDRI 上均匀采样光源方向
            float3 light_dir;
            float env_pdf;
            sampleEnvMapUniform(payload.random_seed, light_dir, env_pdf);

            // 检查光源方向是否在表面上方
            const float ndotl = dot(payload.info.normal, light_dir);
            if (ndotl > 0.f)
            {
                // 构造阴影光线
                RayDesc shadow_ray_desc;
                shadow_ray_desc.Origin = payload.info.position + 0.001f * payload.info.normal;
                shadow_ray_desc.Direction = light_dir;
                shadow_ray_desc.TMin = 0.001f;
                shadow_ray_desc.TMax = 10000.0f;

                // 检测可见性
                if (shadow_ray(rt::rt_tlas, shadow_ray_desc))
                {
                    // 可见：计算直接光照贡献
                    const float2 env_uv = dir_to_env_uv(light_dir);
                    const float3 Li = bindless_srv::sample(gpu_scene.sky, env_uv, gpu_scene.sky_sampler_type).xyz * 8.f;

                    const float3 view_dir = -ray.Direction;
                    const float3 brdf_cos = evalBrdf(payload.info, view_dir, light_dir);
                    const float brdf_pdf = evalBrdfPdf(payload.info, view_dir, light_dir);

                    // MIS 权重（光源采样策略）
                    const float mis_weight = MIS::powerHeuristic(env_pdf, brdf_pdf);

                    // 累加 NEE 贡献: throughput * Li * brdf * cos / env_pdf * mis_weight
                    const float3 nee_contrib = throughput * Li * brdf_cos / max(env_pdf, 1e-7f) * mis_weight;
                    radiance += nee_contrib;
                    nee_radiance += nee_contrib; // 调试通道 4
                }
            }
        }

        // ================================================================
        // 处理材质，计算下一条光线（BRDF 采样）
        // ================================================================
        float3 next_origin, next_dir, mat_throughput, emission;
        bool done;
        float brdf_pdf;
        processMaterial(payload, payload.info, ray.Direction, next_origin, next_dir, mat_throughput, emission, done, brdf_pdf);

        // 累积自发光（如 SKY 材质类型）
        radiance += emission * throughput;

        if (done)
        {
            break;
        }

        // 更新 throughput
        throughput *= mat_throughput;

        // 记录当前路径信息供下次 MIS 使用
        prev_brdf_pdf = brdf_pdf;
        prev_is_delta = payload.info.is_delta_path();

        // 检测是否遇到了 diffuse（在 BRDF 采样之后）
        if (payload.info.material_type == MaterialType::DIFFUSE)
        {
            has_diffuse = true;
        }

        // 俄罗斯轮盘赌
        bool enable_rr = (depth >= 2) && (has_diffuse || luminance(throughput) < 0.2f);
        if (enable_rr)
        {
            float p = max(throughput.x, max(throughput.y, throughput.z));
            p = min(0.95f, p);
            if (Random::rnd(payload.random_seed) > p)
            {
                break;
            }
            throughput /= p;
        }

        // 设置下一条光线
        ray.Origin = next_origin;
        ray.Direction = next_dir;
    }

    // 根据调试通道选择输出
    float3 output_radiance = radiance;
    if (push_const.channel == 4)
    {
        output_radiance = nee_radiance; // NEE 命中 HDRI 的贡献
    }
    else if (push_const.channel == 5)
    {
        output_radiance = emissive_radiance; // 自发光物体的贡献
    }
    else if (push_const.channel == 6)
    {
        output_radiance = hdri_radiance; // 最终命中 HDRI 的贡献
    }

    // 累积帧混合
    if (accum_samples > 0 && push_const.channel == 0)
    {
        float a = 1.f / float(accum_samples + 1);
        const float3 old_color = rt::rt_color.Load(thread_id).xyz;
        rt::rt_color.Store(thread_id, float4(lerp(old_color, output_radiance, a), 1.f));
    }
    else
    {
        rt::rt_color.Store(thread_id, float4(output_radiance, 1.f));
    }
}

