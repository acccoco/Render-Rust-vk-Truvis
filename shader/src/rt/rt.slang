#include "pass/rt.slangi"
#include "pbr.slangi"
#include "sample/random.slangi"
#include "sample/sample.slangi"
#include "scene.slangi"

const static uint max_depth = 16;

/// 超过一定数量后，就不再重新计算了
const static uint max_accum_samples = 4096;

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
    /// 进入剖切体时 +1，离开剖切体时 -1
    int section_stack;
    /// 在剖切体内，进入物体时 +1，离开物体时 -1
    int object_stack;
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
    const float2 subpixel_jitter = (accum_samples == 0 || push_const.channel != 0)
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
    payload.section_stack = 0;
    payload.object_stack = 0;

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

    if (accum_samples > 0 && push_const.channel == 0)
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
    const uint3 triangle = geometry.get_triangle(primitive_id);
    const float3 interp_pos = geometry.get_interp_position(triangle, attr.barycentrics);
    const float3 interp_normal = geometry.get_interp_normal(triangle, attr.barycentrics);
    const float2 interp_uv = geometry.get_interp_uv(triangle, attr.barycentrics);

    const float3 world_pos = mul(ObjectToWorld3x4(), float4(interp_pos, 1.f));

    // 双面材质：确保每次光线都命中物体的正面
    float3 origin_world_normal;
    {
        // normal mat = obejct mat 的逆转置
        Instance *instance = scene.get_instance(instance_id);
        const float4x4 normal_matrix = transpose(instance.inv_model);
        origin_world_normal = normalize(mul(normal_matrix, float4(interp_normal, 0.f)).xyz);
    }
    const float3 world_normal = faceforward(origin_world_normal, WorldRayDirection(), origin_world_normal);

    float3 base_color = float3(0.f, 0.f, 0.f);
    {
        MatPayload mat_payload = {};
        mat_payload.instance_id = instance_id;
        mat_payload.geometry_id = geometry_id;
        mat_payload.uv = interp_uv;
        diffuse_callable(mat_payload);
        base_color = mat_payload.out_color;
    }
    float mat_roughness = mat.roughness;
    float mat_metallic = mat.metallic;
    float mat_opaque = mat.opaque;

    // 法线着色
    if (push_const.channel == 1)
    {
        payload.radiance = world_normal * 0.5f + 0.5f;
        payload.done = true;
        return;
    }
    if (push_const.channel == 2)
    {
        payload.radiance = base_color;
        payload.done = true;
        return;
    }

    // FIXME 临时方案
    // 剖切体: 自发光 > 100.f 就视为剖切体
    if (mat.emissive.x > 100.f)
    {
        payload.section_stack += dot(WorldRayDirection(), origin_world_normal) < 0.f ? 1 : -1;
        if (payload.section_stack == 0)
        { // 离开剖切体，但是在物体内部，设为填充面
            if (payload.object_stack > 0)
            {
                payload.done = true;
                payload.radiance = float3(0.8f, 0.1f, 0.8f); // 紫色
                return;
            }
            else
            {
                payload.radiance = 0.f;
                payload.weight = 1.f;
                payload.ray_origin = world_pos + 0.0001 * WorldRayDirection();
                payload.ray_dir = WorldRayDirection();
                return;
            }
        }
        else
        { // 仍然在剖切体内部
            payload.radiance = 0.f;
            payload.weight = 1.f;
            payload.ray_origin = world_pos + 0.0001 * WorldRayDirection();
            payload.ray_dir = WorldRayDirection();
            return;
        }
    }
    else if (mat.emissive.x > 0.f) // 普通的自发光材质
    {
        payload.radiance = mat.emissive * base_color;
        payload.done = true;
        return;
    }
    else
    {
        // 在剖切体内部
        if (payload.section_stack > 0)
        {
            payload.radiance = 0.f;
            payload.weight = 1.f;
            payload.ray_origin = world_pos + 0.0001 * WorldRayDirection();
            payload.ray_dir = WorldRayDirection();
            payload.object_stack += dot(WorldRayDirection(), origin_world_normal) < 0.f ? 1 : -1;
            return;
        }
    }

    const bool choose_reflect = Random::rnd(payload.random_seed) < mat_opaque;
    if (choose_reflect)
    {
        // brdf = diffuse_brdf + specular_brdf
        const float prob_diffuse = mat_roughness;
        const bool choose_diffuse = Random::rnd(payload.random_seed) < prob_diffuse;
        if (choose_diffuse)
        {
            // 半球 cos 采样
            // BRDF = diffuse_color / pi
            // pdf = cos(theta) / pi
            // weight = BRDF * cos(theta) / pdf = diffuse_color
            const float3 local_dir = Sample::get_cos_hemisphere_sample(payload.random_seed);
            float3 tangent, bitangent;
            Sample::create_local_coord(world_normal, tangent, bitangent);
            payload.ray_dir = normalize(local_dir.x * tangent + local_dir.y * bitangent + local_dir.z * world_normal);

            const float3 diffuse_color = (1.f - mat_metallic) * base_color;
            payload.weight = diffuse_color / prob_diffuse;
        }
        else
        {
            const float reflectance = 0.8f;
            const float3 f0 = lerp(0.16 * reflectance * reflectance, base_color, mat_metallic);
            float real_roughness = mat_roughness * mat_roughness;
            // 首先根据 NDF 采样半程向量 H
            float3 h = SpecularBrdf::ggx_sample(payload.random_seed, real_roughness, world_normal);
            SpecularBrdf specular_brdf = SpecularBrdf(f0, real_roughness, world_normal, -WorldRayDirection(), h);

            payload.weight = specular_brdf.brdf_with_pdf() * specular_brdf.get_ndotl() / (1.f - prob_diffuse);
            payload.ray_dir = specular_brdf.light;
        }

        // 光线的起点增加一个偏移量，避免自交
        payload.ray_origin = world_pos + 0.0001 * payload.ray_dir;
        payload.done = false;
    }
    else
    {
        payload.done = false;
        const float mat_refractive = 1.5f;
        const float refractive = dot(WorldRayDirection(), origin_world_normal) > 0.f ? mat_refractive : 1.f / mat_refractive;
        payload.ray_dir = refract(WorldRayDirection(), world_normal, refractive);
        if (length(payload.ray_dir) < 1e-6f)
        {
            // 折射失败，直接返回
            payload.done = true;
            return;
        }
        payload.weight = base_color;
        payload.ray_origin = world_pos + 0.0001 * payload.ray_dir;
    }
}

[shader("callable")]
void diffuse_callable(inout MatPayload payload)
{
    PBRMaterial *mat = push_const.scene.get_material(payload.instance_id, payload.geometry_id);
    if (mat.diffuse_map.is_valid())
    {
        payload.out_color = mat.diffuse_map.sample_level(payload.uv, 0.0).xyz;
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
    const uint3 triangle = geometry.get_triangle(primitive_id);
    const float2 interp_uv = geometry.get_interp_uv(triangle, attr.barycentrics);

    // 当前命中位置 diffuse 贴图是透的，直接忽略该命中
    const float4 base_color = mat.diffuse_map.sample_level(interp_uv, 0.0);
    if (base_color.w == 0.f)
    {
        IgnoreHit();
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
    payload.radiance = float3(push_const.scene.sky.sample_level(uv, 0.0).xyz) * 8.f;
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
