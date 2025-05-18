// set 1, binding 0
[[vk::binding(0, 1)]]
RWTexture2D<float4> render_target;

// set 1, binding 1
[[vk::binding(1, 1)]]
RaytracingAccelerationStructure tlas;

[shader("raygeneration")]
void main_ray_gen() {
    uint2 thread_id = DispatchRaysIndex().xy;

    
}