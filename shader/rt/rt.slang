#include "scene.slangi"

[[vk::push_constant]]
PushConstants push_const;

struct SkyMissPayload
{
    float4 color;
};

struct ShadowMissPayload
{
    bool miss;
};

[shader("raygeneration")]
void main_ray_gen()
{
    uint2 thread_id = DispatchRaysIndex().xy;

    PerFrameData *frame_data = push_const.frame_data;

    const float2 pixel_center = float2(thread_id) + float2(0.5f, 0.5f);
    const float2 in_uv = pixel_center / float2(DispatchRaysDimensions().xy);
    const float4 target_in_view = mul(frame_data.inv_projection, float4(in_uv.x * 2.0 - 1.0, 1.0 - in_uv.y * 2.0, 1.0, 1.0));
    const float4 direction = mul(frame_data.inv_view, float4(normalize(target_in_view.xyz), 0.0));

    RayDesc ray;
    ray.Origin = frame_data.camera_pos;
    ray.Direction = direction.xyz;
    ray.TMin = 0.001f;
    ray.TMax = 10000.0f;

    SkyMissPayload payload = { float4(0, 0, 0, 1) };

    Scene *scene = push_const.scene;
    RaytracingAccelerationStructure tlas_ = RaytracingAccelerationStructure(scene.tlas);

    TraceRay(tlas_, RAY_FLAG_FORCE_OPAQUE, 0xFF, //
             0,                                  // sbt record offset
             0,                                  // sbt record stride
             0,                                  // sky miss shader
             ray, payload);

    frame_data.rt_render_target.store(thread_id, payload.color);
}

// static const float3 directional_light = normalize(float3(0.577f, -0.577f, -0.577f)); // 方向光
static const float3 directional_light = normalize(float3(0.f, -1.f, -0.f)); // 方向光

[shader("closesthit")]
void main_closest_hit(inout SkyMissPayload payload, in BuiltInTriangleIntersectionAttributes attr)
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

        const uint ray_flags = RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER;
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

    const float4 tex_color = mat.diffuse_map.sample(interp_vertex.uv);
    payload.color = float4(tex_color.xyz * attenuation, tex_color.w);
}

[shader("miss")]
void sky_miss(inout SkyMissPayload payload)
{
    payload.color = float4(0.8f, 0.8f, 0.8f, 1.0f); // 蓝色背景
}

[shader("miss")]
void shadow_miss(inout ShadowMissPayload payload)
{
    payload.miss = true;
}
