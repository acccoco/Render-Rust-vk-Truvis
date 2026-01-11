/// @file rt_hit.slang
/// @brief 命中着色器（Closest Hit + Any Hit + Callable）
///
/// 实现路径追踪的材质计算、BRDF 采样和透明度处理

#include "./payload.slangi"
#include "lib/bindless_op.slangi"
#include "lib/pbr.slangi"
#include "lib/sample/random.slangi"
#include "lib/sample/sample.slangi"

// ============================================================================
// Callable Shader
// ============================================================================

/// 漫反射颜色采样
/// 根据材质的 diffuse_map 或 base_color 返回漫反射颜色
[shader("callable")]
void diffuse_callable(inout MatPayload payload)
{
    PBRMaterial* mat = gpu_scene.get_material(payload.instance_id, payload.geometry_id);
    if (bindless_srv::is_valid(mat.diffuse_map))
    {
        payload.out_color = bindless_srv::sample_level(mat.diffuse_map, payload.uv, mat.diffuse_map_sampler_type, 0.0).xyz;
    }
    else
    {
        payload.out_color = mat.base_color;
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 处理剖切体逻辑
/// @param payload 光线载荷
/// @param mat 材质
/// @param world_pos 世界空间位置
/// @param origin_world_normal 原始世界法线（未翻转）
/// @return true 表示已处理完成，false 表示继续正常着色
bool handleSectionPlane(
    inout HitPayload payload,
    PBRMaterial* mat,
    float3 world_pos,
    float3 origin_world_normal
)
{
    // 剖切体: 自发光 > 100.f 就视为剖切体
    if (mat.emissive.x > 100.f)
    {
        payload.section_stack += dot(WorldRayDirection(), origin_world_normal) < 0.f ? 1 : -1;
        if (payload.section_stack == 0)
        {
            // 离开剖切体，但是在物体内部，设为填充面
            if (payload.object_stack > 0)
            {
                payload.done = true;
                payload.emission = float3(0.8f, 0.1f, 0.8f); // 紫色
            }
            else
            {
                payload.emission = 0.f;
                payload.throughput = 1.f;
                payload.ray_origin = world_pos + 0.0001 * WorldRayDirection();
                payload.ray_dir = WorldRayDirection();
            }
        }
        else
        {
            // 仍然在剖切体内部
            payload.emission = 0.f;
            payload.throughput = 1.f;
            payload.ray_origin = world_pos + 0.0001 * WorldRayDirection();
            payload.ray_dir = WorldRayDirection();
        }
        return true;
    }
    return false;
}

/// 处理自发光材质
/// @param payload 光线载荷
/// @param mat 材质
/// @param base_color 基础颜色
/// @return true 表示是自发光材质
bool handleEmissive(
    inout HitPayload payload,
    PBRMaterial* mat,
    float3 base_color
)
{
    if (mat.emissive.x > 0.f)
    {
        payload.emission = mat.emissive * base_color;
        payload.done = true;
        return true;
    }
    return false;
}

/// 处理剖切体内部的物体
/// @param payload 光线载荷
/// @param world_pos 世界空间位置
/// @param origin_world_normal 原始世界法线
/// @return true 表示在剖切体内部
bool handleInsideSection(
    inout HitPayload payload,
    float3 world_pos,
    float3 origin_world_normal
)
{
    if (payload.section_stack > 0)
    {
        payload.emission = 0.f;
        payload.throughput = 1.f;
        payload.ray_origin = world_pos + 0.0001 * WorldRayDirection();
        payload.ray_dir = WorldRayDirection();
        payload.object_stack += dot(WorldRayDirection(), origin_world_normal) < 0.f ? 1 : -1;
        return true;
    }
    return false;
}

/// 处理漫反射采样
/// @param payload 光线载荷
/// @param world_normal 世界法线
/// @param base_color 基础颜色
/// @param mat_metallic 金属度
/// @param prob_diffuse 漫反射概率（用于重要性采样补偿）
void sampleDiffuse(
    inout HitPayload payload,
    float3 world_normal,
    float3 base_color,
    float mat_metallic,
    float prob_diffuse
)
{
    // 半球 cos 采样
    // BRDF = diffuse_color / pi
    // pdf = cos(theta) / pi
    // throughput  = BRDF * cos(theta) / pdf = diffuse_color
    const float3 local_dir = Sample::get_cos_hemisphere_sample(payload.random_seed);
    float3 tangent, bitangent;
    Sample::create_local_coord(world_normal, tangent, bitangent);
    payload.ray_dir = normalize(local_dir.x * tangent + local_dir.y * bitangent + local_dir.z * world_normal);

    const float3 diffuse_color = (1.f - mat_metallic) * base_color;
    payload.throughput = diffuse_color / prob_diffuse;
}

/// 处理镜面反射采样
/// @param payload 光线载荷
/// @param world_normal 世界法线
/// @param base_color 基础颜色
/// @param mat_metallic 金属度
/// @param mat_roughness 粗糙度
/// @param prob_specular 镜面反射概率（1 - prob_diffuse）
void sampleSpecular(
    inout HitPayload payload,
    float3 world_normal,
    float3 base_color,
    float mat_metallic,
    float mat_roughness,
    float prob_specular
)
{
    const float reflectance = 0.8f;
    const float3 f0 = lerp(0.16 * reflectance * reflectance, base_color, mat_metallic);
    float real_roughness = mat_roughness * mat_roughness;

    // 根据 NDF 采样半程向量 H
    float3 h = SpecularBrdf::ggx_sample(payload.random_seed, real_roughness, world_normal);
    SpecularBrdf specular_brdf = SpecularBrdf(f0, real_roughness, world_normal, -WorldRayDirection(), h);

    payload.throughput = specular_brdf.brdf_with_pdf() * specular_brdf.get_ndotl() / prob_specular;
    payload.ray_dir = specular_brdf.light;
}

/// 处理折射
/// @param payload 光线载荷
/// @param world_pos 世界空间位置
/// @param world_normal 世界法线（已翻转）
/// @param origin_world_normal 原始世界法线
/// @param base_color 基础颜色
/// @return true 表示折射成功
bool sampleRefraction(
    inout HitPayload payload,
    float3 world_pos,
    float3 world_normal,
    float3 origin_world_normal,
    float3 base_color
)
{
    const float mat_refractive = 1.5f;
    const float refractive = dot(WorldRayDirection(), origin_world_normal) > 0.f ? mat_refractive : 1.f / mat_refractive;

    payload.ray_dir = refract(WorldRayDirection(), world_normal, refractive);
    if (length(payload.ray_dir) < 1e-6f)
    {
        // 全内反射，折射失败
        payload.done = true;
        return false;
    }

    payload.throughput = base_color;
    payload.ray_origin = world_pos + 0.0001 * payload.ray_dir;
    payload.done = false;
    return true;
}

// ============================================================================
// Closest Hit Shader
// ============================================================================

[shader("closesthit")]
void main_closest_hit(inout HitPayload payload, in BuiltInTriangleIntersectionAttributes attr)
{
    const uint instance_id = InstanceIndex();
    const uint geometry_id = GeometryIndex();
    const uint primitive_id = PrimitiveIndex();

    // 获取材质和几何体数据
    PBRMaterial* mat = gpu_scene.get_material(instance_id, geometry_id);
    Geometry* geometry = gpu_scene.get_geometry(instance_id, geometry_id);
    const uint3 triangle = geometry.get_triangle(primitive_id);
    const float3 interp_pos = geometry.get_interp_position(triangle, attr.barycentrics);
    const float3 interp_normal = geometry.get_interp_normal(triangle, attr.barycentrics);
    const float2 interp_uv = geometry.get_interp_uv(triangle, attr.barycentrics);

    const float3 world_pos = mul(ObjectToWorld3x4(), float4(interp_pos, 1.f));

    // 双面材质：确保每次光线都命中物体的正面
    float3 origin_world_normal;
    {
        Instance* instance = gpu_scene.get_instance(instance_id);
        const float4x4 normal_matrix = transpose(instance.inv_model);
        origin_world_normal = normalize(mul(normal_matrix, float4(interp_normal, 0.f)).xyz);
    }
    const float3 world_normal = faceforward(origin_world_normal, WorldRayDirection(), origin_world_normal);

    // 获取基础颜色
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

    // 调试通道：法线着色
    if (push_const.channel == 1)
    {
        payload.emission = world_normal * 0.5f + 0.5f;
        payload.done = true;
        return;
    }

    // 调试通道：基础颜色
    if (push_const.channel == 2)
    {
        payload.emission = base_color;
        payload.done = true;
        return;
    }

    // 处理剖切体
    if (handleSectionPlane(payload, mat, world_pos, origin_world_normal))
    {
        return;
    }

    // 处理自发光材质
    if (handleEmissive(payload, mat, base_color))
    {
        return;
    }

    // 处理剖切体内部的物体
    if (handleInsideSection(payload, world_pos, origin_world_normal))
    {
        return;
    }

    // BRDF 采样
    const bool choose_reflect = Random::rnd(payload.random_seed) < mat_opaque;
    if (choose_reflect)
    {
        // 反射路径：漫反射 + 镜面反射
        const float prob_diffuse = mat_roughness;
        const bool choose_diffuse = Random::rnd(payload.random_seed) < prob_diffuse;

        if (choose_diffuse)
        {
            sampleDiffuse(payload, world_normal, base_color, mat_metallic, prob_diffuse);
        }
        else
        {
            sampleSpecular(payload, world_normal, base_color, mat_metallic, mat_roughness, 1.f - prob_diffuse);
        }

        // 光线的起点增加一个偏移量，避免自交
        payload.ray_origin = world_pos + 0.0001 * payload.ray_dir;
        payload.done = false;
    }
    else
    {
        // 折射路径
        sampleRefraction(payload, world_pos, world_normal, origin_world_normal, base_color);
    }
}

// ============================================================================
// Any Hit Shader
// ============================================================================

/// 根据 texture 的透明度，判断光线是否应该穿过
[shader("anyhit")]
void trans_any(inout HitPayload payload, in BuiltInTriangleIntersectionAttributes attr)
{
    const uint instance_id = InstanceIndex();
    const uint geometry_id = GeometryIndex();
    const uint primitive_id = PrimitiveIndex();

    PBRMaterial* mat = gpu_scene.get_material(instance_id, geometry_id);
    Geometry* geometry = gpu_scene.get_geometry(instance_id, geometry_id);
    const uint3 triangle = geometry.get_triangle(primitive_id);
    const float2 interp_uv = geometry.get_interp_uv(triangle, attr.barycentrics);

    // 当前命中位置 diffuse 贴图是透的，直接忽略该命中
    const float4 base_color = bindless_srv::sample_level(mat.diffuse_map, interp_uv, mat.diffuse_map_sampler_type, 0.0);
    if (base_color.w == 0.f)
    {
        IgnoreHit();
    }
}
