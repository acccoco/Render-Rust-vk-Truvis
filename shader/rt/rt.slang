// set 1, binding 0
[[vk::binding(0, 1)]]
RWTexture2D<float4> render_target;

// set 1, binding 1
[[vk::binding(1, 1)]]
RaytracingAccelerationStructure tlas;

struct RayPayload
{
    float4 color;
};

[shader("raygeneration")]
void main_ray_gen() {
    uint2 thread_id = DispatchRaysIndex().xy;

    RayDesc ray;
    ray.Origin = float3(0, 0, 0);
    ray.Direction = float3(0, 0, 1);
    ray.TMin = 0.001f;
    ray.TMax = 10000.0f;
    RayPayload payload = {float4(0, 0, 0, 1)};
    TraceRay(tlas, RAY_FLAG_NONE, 0xFF, 0, 0, 0, ray, payload);
}