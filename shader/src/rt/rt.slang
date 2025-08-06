#include "pass/rt.slangi"
#include "pbr.slangi"
#include "sample/random.slangi"
#include "sample/sample.slangi"
#include "scene.slangi"

const static uint max_depth = 10;

/// 超过一定数量后，就不再重新计算了
const static uint max_accum_samples = 1024;

[[vk::push_constant]]
rt::PushConstants push_const;

struct HitPayload
{
    /// 命中点的自发光颜色
    float3 radiance;
    /// 当前命中位置的衰减：BRDF * cos(theta) / pdf
    float3 weight;
    /// 是否需要提前中止
    bool done;
    /// 下一次光线的起点
    float3 ray_origin;
    /// 下一次光线的方向
    float3 ray_dir;
    uint random_seed;
};

struct ShadowMissPayload
{
    bool miss;
};

struct MatPayload
{
    uint instance_id;
    uint geometry_id;
    float2 uv;
    float3 out_color;
};

[shader("raygeneration")]
void main_ray_gen()
{
    PerFrameData *frame_data = push_const.frame_data;
    const uint accum_samples = frame_data.accum_frames * push_const.spp + push_const.spp_idx;
    if (accum_samples >= max_accum_samples)
    {
        // 达到最大累积帧数，直接返回
        return;
    }

    uint2 thread_id = DispatchRaysIndex().xy;

    // 下方的 0.375 来自于 Blender 的 Cycles
    uint random_seed = Random::tea(thread_id.y * DispatchRaysDimensions().x + thread_id.x, accum_samples);
    const float2 subpixel_jitter = accum_samples == 0
                                       ? float2(0.5f)
                                       : float2(0.5) + 0.375 * Random::rand_gaussian(random_seed);

    // 以 thread_id + 0.5 为中心，进行子像素抖动
    const float2 pixel_center = float2(thread_id) + subpixel_jitter;
    const float2 in_uv = pixel_center / float2(DispatchRaysDimensions().xy);
    const float4 target_in_view = mul(frame_data.inv_projection, float4(in_uv.x * 2.0 - 1.0, 1.0 - in_uv.y * 2.0, 1.0, 1.0));
    const float4 direction = mul(frame_data.inv_view, float4(normalize(target_in_view.xyz), 0.0));

    RayDesc ray;
    ray.Origin = frame_data.camera_pos;
    ray.Direction = direction.xyz;
    ray.TMin = 0.001f;
    ray.TMax = 10000.0f;

    HitPayload payload = {};
    payload.radiance = float3(0.0f, 0.0f, 0.0f); // 初始化颜色
    payload.weight = float3(1.0f, 1.0f, 1.0f);
    payload.done = false;
    payload.random_seed = random_seed;

    Scene *scene = push_const.scene;
    RaytracingAccelerationStructure tlas_ = RaytracingAccelerationStructure(scene.tlas);

    // E 直接光照
    // w = BRDF * cos(theta) / pdf
    // Lo = w_0 * Li_0 + E_0
    // Li_0 = w_1 * Li_1 + E_1
    // Lo = E_0 + w_0 * E_1 + w_0 * w_1 * E_2 + ...
    //      + w_0 * ... * w_n * Li_n
    float3 color = float3(0.0f, 0.0f, 0.0f);
    float3 weight = float3(1.f, 1.f, 1.f);
    for (uint depth = 0; depth < max_depth && !payload.done; ++depth)
    {
        TraceRay(tlas_, RAY_FLAG_NONE, 0xFF, //
                 0,                          // sbt record offset
                 0,                          // sbt record stride
                 0,                          // sky miss shader
                 ray, payload);

        color += payload.radiance * weight;
        weight *= payload.weight;
        ray.Origin = payload.ray_origin;
        ray.Direction = payload.ray_dir;
    }
    // TODO remove me：应该是计算出问题了，这里只是补救措施
    color = clamp(color, float3(0.f), float3(100.f));

    if (accum_samples > 0)
    {
        float a = 1.f / float(accum_samples + 1);
        const float3 old_color = push_const.rt_render_target.load(thread_id).xyz;
        // const float3 old_color = float3(1.f, 0.f, 0.f);
        push_const.rt_render_target.store(thread_id, float4(lerp(old_color, color, a), 1.f));
    }
    else
    {
        push_const.rt_render_target.store(thread_id, float4(color, 1.f));
    }
}

static const float3 directional_light = normalize(float3(0.1f, -0.98995f, -0.1f)); // 方向光

[shader("closesthit")]
void main_closest_hit(inout HitPayload payload, in BuiltInTriangleIntersectionAttributes attr)
{
    PerFrameData *frame_data = push_const.frame_data;
    Scene *scene = push_const.scene;

    const uint instance_id = InstanceIndex();
    const uint geometry_id = GeometryIndex();
    const uint primitive_id = PrimitiveIndex();

    PBRMaterial *mat = scene.get_material(instance_id, geometry_id);
    Geometry *geometry = scene.get_geometry(instance_id, geometry_id);
    const Vertex interp_vertex = geometry.get_interp_vertex(primitive_id, attr.barycentrics);
    const float3 world_pos = mul(ObjectToWorld3x4(), float4(interp_vertex.position, 1.f));

    // 双面材质：确保每次光线都命中物体的正面
    const float3 origin_world_normal = normalize(mul(interp_vertex.normal, ObjectToWorld3x4()).xyz);
    const float3 world_normal = faceforward(origin_world_normal, WorldRayDirection(), origin_world_normal);

    // TODO 材质参数
    const float metallic = 0.8f;
    const float perceptual_roughness = 0.1f;
    const float reflectance = 0.5f;
    float3 base_color = float3(0.f, 0.f, 0.f);
    {
        MatPayload mat_payload = {};
        mat_payload.instance_id = instance_id;
        mat_payload.geometry_id = geometry_id;
        mat_payload.uv = interp_vertex.uv;
        diffuse_callable(mat_payload);
        base_color = mat_payload.out_color;
    }

    // brdf = diffuse_brdf + specular_brdf
    const float prob_diffuse = 0.2f;
    const bool choose_diffuse = Random::rnd(payload.random_seed) < prob_diffuse;
    if (choose_diffuse)
    {
        // 半球 cos 采样
        // BRDF = diffuse_color / pi
        // pdf = cos(theta) / pi
        // weight = BRDF * cos(theta) / pdf = diffuse_color
        {
            const float3 local_dir = Sample::get_cos_hemisphere_sample(payload.random_seed);
            float3 tangent, bitangent;
            Sample::create_local_coord(world_normal, tangent, bitangent);
            payload.ray_dir = normalize(local_dir.x * tangent + local_dir.y * bitangent + local_dir.z * world_normal);

            const float3 diffuse_color = (1.f - metallic) * base_color;
            payload.weight = diffuse_color;
        }
        payload.weight /= prob_diffuse;
    }
    else
    {
        const float3 f0 = lerp(0.16 * reflectance * reflectance, base_color, metallic);
        const float roughness = perceptual_roughness * perceptual_roughness;
        // 首先根据 NDF 采样半程向量 H
        const float3 h = SpecularBrdf::ggx_sample(payload.random_seed, mat.roughness, world_normal);
        SpecularBrdf specular_brdf = SpecularBrdf(f0, roughness, world_normal, -WorldRayDirection(), h);
        const float brdf_pdf = specular_brdf.ggx_pdf();

        payload.weight = specular_brdf.brdf_with_pdf() * specular_brdf.get_ndotl() / (1.f - prob_diffuse);
        // payload.weight = specular_brdf.brdf() * specular_brdf.get_ndotl() / (1.f * (1.f - prob_diffuse));
        payload.weight = max(0.001f, payload.weight);
    }

    // 光线的起点增加一个偏移量，避免自交
    payload.ray_origin = world_pos + 0.0001 * world_normal;
    payload.done = false;
}

[shader("callable")]
void diffuse_callable(inout MatPayload payload)
{
    PBRMaterial *mat = push_const.scene.get_material(payload.instance_id, payload.geometry_id);
    if (mat.diffuse_map.is_valid())
    {
        payload.out_color = mat.diffuse_map.sample(payload.uv).xyz * mat.base_color;
    }
    else
    {
        payload.out_color = mat.base_color;
    }
}

/// 根据 texture 的透明度，判断光线是否应该穿过
[shader("anyhit")]
void trans_any(inout HitPayload payload, in BuiltInTriangleIntersectionAttributes attr)
{
    PerFrameData *frame_data = push_const.frame_data;
    Scene *scene = push_const.scene;

    const uint instance_id = InstanceIndex();
    const uint geometry_id = GeometryIndex();
    const uint primitive_id = PrimitiveIndex();

    PBRMaterial *mat = scene.get_material(instance_id, geometry_id);
    Geometry *geometry = scene.get_geometry(instance_id, geometry_id);
    const Vertex interp_vertex = geometry.get_interp_vertex(primitive_id, attr.barycentrics);

    // 当前命中位置 diffuse 颜色
    const float4 base_color = mat.diffuse_map.sample(interp_vertex.uv);
    if (base_color.w == 0.f)
    {
        IgnoreHit();
    }
    else
    {
        return;
    }
}

/// right-hand, -z front, y up
/// phi = 0, -z
/// phi = 90, -x
/// @return phi: [-pi, pi], theta: [-pi/2, pi/2]
float2 dir_to_angle(const float3 dir)
{
    // 将方向向量转换为角度
    const float phi = atan2(dir.x, dir.z);
    const float theta = asin(dir.y);
    return float2(phi, theta);
}

[shader("miss")]
void sky_miss(inout HitPayload payload)
{
    const float2 angle = dir_to_angle(WorldRayDirection());
    // phi: [-pi, pi] -> [0, 1]
    // theta: [-pi/2, pi/2] -> [1, 0]
    const float2 uv = float2(angle.x / M_PI * 0.5 + 0.5, 0.5 - angle.y / M_PI);
    payload.radiance = float3(push_const.scene.sky.sample(uv).xyz) * 8.f;
    payload.done = true;
}

// shadow miss 的 flag
// const uint ray_flags =                         //
//     RAY_FLAG_FORCE_OPAQUE                      // 强制不透明
//     | RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH // 接受第一个命中并结束搜索
//     | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER         // 跳过 closest hit shader
//     ;
[shader("miss")]
void shadow_miss(inout ShadowMissPayload payload)
{
    payload.miss = true;
}
