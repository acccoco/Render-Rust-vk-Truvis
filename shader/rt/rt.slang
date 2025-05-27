#include "../include/scene.slangi"


[[vk::push_constant]]
PushConstants push_const;

// set 0, binding 0
// TODO 可能需要 nonuniform，表示同一个 wavefront 的下标是不同的
[[vk::binding(0, 0)]]
Sampler2D<float4> textures[];

// set 0, binding 1
[[vk::binding(1, 0)]]
Texture2D<float4> images[];

struct RayPayload
{
    float4 color;
};

[shader("raygeneration")]
void main_ray_gen() {
    uint2 thread_id = DispatchRaysIndex().xy;

    PerFrameData* frame_data = push_const.frame_data;

    const float2 pixel_center = float2(thread_id) + float2(0.5f, 0.5f);
    const float2 in_uv = pixel_center / float2(DispatchRaysDimensions().xy);
    const float4 target_in_view = mul(frame_data.inv_projection, float4(in_uv * 2.0 - 1.0, 1.0, 1.0));
    const float4 direction = mul(frame_data.inv_view, float4(normalize(target_in_view.xyz), 0.0));

    RayDesc ray;
    ray.Origin = frame_data.camera_pos;
    ray.Direction = direction.xyz;
    ray.TMin = 0.001f;
    ray.TMax = 10000.0f;

    RayPayload payload = {float4(0, 0, 0, 1)};

    Scene* scene = push_const.scene;
    RaytracingAccelerationStructure tlas_ = RaytracingAccelerationStructure(scene.tlas);

    TraceRay(tlas_, RAY_FLAG_FORCE_OPAQUE, 0xFF, 0, 0, 0, ray, payload);

    frame_data.rt_render_target.store(thread_id, payload.color);
}

[shader("closesthit")]
void main_closest_hit(inout RayPayload payload, in BuiltInTriangleIntersectionAttributes attr) {
    PerFrameData* frame_data = push_const.frame_data;
    Scene* scene = push_const.scene;

    const uint instance_id = InstanceIndex();
    const int geometry_id = GeometryIndex();

    payload.color = float4((instance_id % 13) / 13.0, (instance_id % 17) / 17.0, (instance_id % 23) / 23.0, 1.0f);
}

[shader("miss")]
void main_miss(inout RayPayload payload) {
    payload.color = float4(0.0f, 0.0f, 1.0f, 1.0f); // 蓝色背景
}