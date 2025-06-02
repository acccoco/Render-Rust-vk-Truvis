#include "pass/rt.slangi"
#include "random.slangi"
#include "scene.slangi"

/// 太幽默了哥，为啥我需要手动限制最大递归深度啊，pipeline 里面的设置算啥？
const static uint max_depth = 8;

/// 超过一定数量后，就不再重新计算了
const static uint max_accum_frames = 100;

[[vk::push_constant]]
rt::PushConstants push_const;

struct MainPayload
{
    float3 color;
    float3 attenuation;
    int done;
    float3 ray_origin;
    float3 ray_dir;
};

struct ShadowMissPayload
{
    bool miss;
};

[shader("raygeneration")]
void main_ray_gen()
{
    PerFrameData *frame_data = push_const.frame_data;
    if (frame_data.accum_frames >= max_accum_frames)
    {
        // 达到最大累积帧数，直接返回
        return;
    }

    uint2 thread_id = DispatchRaysIndex().xy;

    Random random = Random(thread_id.y * DispatchRaysDimensions().x + thread_id.x, frame_data.accum_frames);
    const float r1 = random.next();
    const float r2 = random.next();
    const float2 subpixel_jitter = frame_data.accum_frames == 0 ? float2(0.5f) : float2(r1, r2);

    const float2 pixel_center = float2(thread_id) + subpixel_jitter;
    const float2 in_uv = pixel_center / float2(DispatchRaysDimensions().xy);
    const float4 target_in_view = mul(frame_data.inv_projection, float4(in_uv.x * 2.0 - 1.0, 1.0 - in_uv.y * 2.0, 1.0, 1.0));
    const float4 direction = mul(frame_data.inv_view, float4(normalize(target_in_view.xyz), 0.0));

    RayDesc ray;
    ray.Origin = frame_data.camera_pos;
    ray.Direction = direction.xyz;
    ray.TMin = 0.001f;
    ray.TMax = 10000.0f;

    MainPayload payload = {};
    payload.color = float3(0.0f, 0.0f, 0.0f); // 初始化颜色
    payload.attenuation = float3(1.0f, 1.0f, 1.0f);
    payload.done = 0;

    Scene *scene = push_const.scene;
    RaytracingAccelerationStructure tlas_ = RaytracingAccelerationStructure(scene.tlas);

    float3 color = float3(0.0f, 0.0f, 0.0f);
    for (uint depth = 0; depth < max_depth && payload.done == 0; ++depth)
    {
        float3 attenuation = payload.attenuation;

        TraceRay(tlas_, RAY_FLAG_NONE, 0xFF, //
                 0,                          // sbt record offset
                 0,                          // sbt record stride
                 0,                          // sky miss shader
                 ray, payload);

        color += payload.color * attenuation;
        ray.Origin = payload.ray_origin;
        ray.Direction = payload.ray_dir;
    }

    if (frame_data.accum_frames > 0)
    {
        float a = 1.f / float(frame_data.accum_frames + 1);
        const float3 old_color = frame_data.rt_render_target.load(thread_id).xyz;
        // const float3 old_color = float3(1.f, 0.f, 0.f);
        frame_data.rt_render_target.store(thread_id, float4(lerp(old_color, color, a), 1.f));
    }
    else
    {
        frame_data.rt_render_target.store(thread_id, float4(color, 1.f));
    }
}

static const float3 directional_light = normalize(float3(0.1f, -0.98995f, -0.1f)); // 方向光

[shader("closesthit")]
void main_closest_hit(inout MainPayload payload, in BuiltInTriangleIntersectionAttributes attr)
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
    const float3 world_normal = normalize(mul(interp_vertex.normal, ObjectToWorld3x4()).xyz);

    float3 tex_color;
    if (mat.diffuse_map.is_valid())
    {
        tex_color = mat.diffuse_map.sample(interp_vertex.uv).xyz;
        payload.attenuation *= 0.4f;
        if (payload.attenuation.x > 0.1f)
        {
            payload.done = 0;
            payload.ray_dir = reflect(WorldRayDirection(), world_normal);
            payload.ray_origin = world_pos + payload.ray_dir * 0.001f; // 偏移一点，避免自交
        }
        else
        {
            payload.done = 1;
        }
    }
    else
    {
        tex_color = float3(mat.reflection);
        payload.attenuation *= 0.7;
        if (payload.attenuation.x > 0.2)
        {
            payload.done = 0;
            payload.ray_dir = reflect(WorldRayDirection(), world_normal);
            payload.ray_origin = world_pos + payload.ray_dir * 0.001f; // 偏移一点，避免自交
        }
        else
        {
            payload.done = 1;
        }
    }

    float attenuation = 1.f;
    if (dot(directional_light, world_normal) > 0.f)
    {
        attenuation = 0.3f;
    }
    else
    {
        RayDesc ray;
        ray.Origin = world_pos - directional_light * 0.001f; // 偏移一点，避免自交
        ray.Direction = -directional_light;
        ray.TMin = 0.001f;
        ray.TMax = 10000.0f;
        ShadowMissPayload shadow_payload = { false };
        RaytracingAccelerationStructure tlas_ = RaytracingAccelerationStructure(scene.tlas);

        const uint ray_flags =                         //
            RAY_FLAG_FORCE_OPAQUE                      // 强制不透明
            | RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH // 接受第一个命中并结束搜索
            | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER         // 跳过 closest hit shader
            ;

        TraceRay(tlas_, ray_flags, 0xFF, //
                 0,                      // sbt record offset
                 0,                      // sbt record stride
                 1,                      // shadow miss shader
                 ray, shadow_payload);

        if (!shadow_payload.miss) // 无法接收到方向光
        {
            attenuation = 0.3f;
        }
    }

    payload.color = float3(mat.base_color * tex_color.xyz * attenuation);
}

/// FIXME 这个效率很低
[shader("anyhit")]
void trans_any(inout MainPayload payload, in BuiltInTriangleIntersectionAttributes attr)
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
    const float4 tex_color = mat.diffuse_map.sample(interp_vertex.uv);
    if (tex_color.w == 0.f)
    {
        IgnoreHit();
    }
    else
    {
        return;
    }
}

const static float PI = 3.14159265358979323846f;

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
void sky_miss(inout MainPayload payload)
{
    const float2 angle = dir_to_angle(WorldRayDirection());
    // phi: [-pi, pi] -> [0, 1]
    // theta: [-pi/2, pi/2] -> [1, 0]
    const float2 uv = float2(angle.x / PI * 0.5 + 0.5, 0.5 - angle.y / PI);
    payload.color = float3(push_const.scene.sky.sample(uv).xyz);
    // payload.color = float4(0.8f, 0.8f, 0.8f, 1.0f); // 蓝色背景
    payload.done = 1; 
}

[shader("miss")]
void shadow_miss(inout ShadowMissPayload payload)
{
    payload.miss = true;
}
