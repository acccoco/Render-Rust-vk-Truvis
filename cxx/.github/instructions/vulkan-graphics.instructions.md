---
description: 'Vulkan graphics programming and rendering engine best practices'
applyTo: '**/*.rs, **/*.slang, **/*.glsl, **/*.hlsl, **/*.slangi, **/*.vert, **/*.frag, **/*.comp, **/*.rgen, **/*.rchit, **/*.rmiss'
---

# Vulkan Graphics Programming Best Practices

Guidelines for developing Vulkan-based rendering engines with Rust, focusing on performance, correctness, and maintainability.

## General Principles

- Follow the Vulkan specification and validation layers for correctness
- Design for GPU parallelism and minimize CPU-GPU synchronization
- Use modern Vulkan features (1.3+) including dynamic rendering, synchronization2, and timeline semaphores
- Prefer bindless/descriptor indexing patterns for flexible resource management
- Profile before optimizing—use RenderDoc, Nsight Graphics, or similar tools

## Vulkan API Guidelines

### Resource Management
- Use RAII patterns for Vulkan objects (wrap in Rust structs with `Drop` implementation)
- Prefer VMA (Vulkan Memory Allocator) or similar for memory management
- Use staging buffers for CPU-to-GPU transfers
- Batch resource updates to minimize pipeline stalls
- Implement proper resource lifetime tracking for frames-in-flight

### Command Buffer Best Practices
- Use one-time submit command buffers for transfer operations
- Reuse command buffers where possible for repeated rendering
- Use secondary command buffers for parallel command recording
- Name command buffers and resources for debugging (VK_EXT_debug_utils)

### Synchronization
- Prefer timeline semaphores over binary semaphores for complex synchronization
- Use pipeline barriers with minimal stage/access masks
- Batch barriers when possible to reduce overhead
- Understand execution and memory dependencies thoroughly
- Use `VK_PIPELINE_STAGE_2_*` and `VK_ACCESS_2_*` flags (synchronization2)

### Descriptor Management
- Use descriptor indexing/bindless for flexible resource access
- Prefer push constants for frequently changing small data
- Use descriptor buffer or descriptor sets efficiently
- Minimize descriptor set switches during rendering

## Shader Development

### General Shader Guidelines
- Write shaders in Slang (preferred), GLSL, or HLSL
- Use `#include` for shared definitions (`.slangi` headers)
- Define clear interfaces between shader stages
- Minimize register pressure and shared memory usage

### Slang-Specific Patterns
```slang
// Use proper type definitions from shared headers
#include "frame_data.slangi"
#include "material.slangi"

// Prefer structured buffer access over raw pointers
StructuredBuffer<Material> materials;

// Use semantic annotations
struct VSInput {
    float3 position : POSITION;
    float3 normal   : NORMAL;
    float2 uv       : TEXCOORD0;
};
```

### SPIR-V Compilation
- Use Slang compiler (`slangc`) for primary shader compilation
- Enable optimization flags for release builds
- Validate SPIR-V output with `spirv-val`
- Generate reflection data for automatic binding generation

## Rendering Architecture

### Frame Structure
- Implement triple buffering (frames-in-flight = 3)
- Use frame labels (A/B/C) for resource identification
- Clear per-frame resources at frame start
- Synchronize with timeline semaphores

### Render Pass Design
- Use dynamic rendering (VK_KHR_dynamic_rendering) over traditional render passes
- Group draws by pipeline state to minimize state changes
- Use attachment load/store operations efficiently
- Consider subpass dependencies for complex passes

### Pipeline Management
- Cache pipeline objects (use pipeline cache)
- Use pipeline libraries for faster compilation
- Prefer dynamic state over pipeline variants where possible
- Document pipeline requirements and assumptions

## Performance Guidelines

### Memory Access Patterns
- Align buffer data to device limits (minUniformBufferOffsetAlignment)
- Use appropriate memory types (device-local vs host-visible)
- Batch small uploads into larger transfers
- Consider memory aliasing for transient resources

### Draw Call Optimization
- Use indirect drawing for GPU-driven rendering
- Implement frustum and occlusion culling
- Batch similar draw calls
- Use instancing for repeated geometry

### Compute Shaders
- Choose workgroup sizes based on occupancy analysis
- Use shared memory for inter-thread communication
- Minimize divergent branching
- Use subgroup operations where available

## Coordinate Systems (Project-Specific)

This project follows specific coordinate system conventions:
- **Model/World**: Right-handed, Y-Up
- **View**: Right-handed, Y-Up, camera looks toward -Z
- **NDC**: Left-handed, Y-Up (Vulkan standard)
- **Framebuffer**: Origin at top-left, use negative viewport height for Y-flip

```rust
// Correct viewport setup for Vulkan Y-flip
let viewport = vk::Viewport {
    x: 0.0,
    y: extent.height as f32,      // Start from bottom
    width: extent.width as f32,
    height: -(extent.height as f32), // Negative height
    min_depth: 0.0,
    max_depth: 1.0,
};
```

## Debugging and Validation

### Validation Layers
- Always enable validation layers during development
- Use VK_LAYER_KHRONOS_validation
- Configure message severity and types appropriately
- Address all validation errors before release

### Debug Utilities
- Name all Vulkan objects with VK_EXT_debug_utils
- Use debug labels for command buffer regions
- Include frame information in names: `[F42A]pass-name`
- Use RenderDoc markers for frame analysis

### Common Pitfalls to Avoid
- Don't submit command buffers that are still recording
- Don't access resources during GPU execution without synchronization
- Don't forget to transition image layouts before use
- Don't ignore alignment requirements for buffer data
- Don't cache `RefCell` borrows across function calls (causes panics)

## Ray Tracing (VK_KHR_ray_tracing_pipeline)

### Acceleration Structures
- Build BLAS for static geometry, update for dynamic
- Use compaction for BLAS to reduce memory
- Rebuild TLAS per frame for dynamic scenes
- Use appropriate build flags (PREFER_FAST_TRACE vs PREFER_FAST_BUILD)

### Shader Binding Table
- Organize SBT entries for efficient access
- Use shader record data for per-instance parameters
- Align SBT entries to device requirements

### Ray Tracing Shaders
```slang
// Ray generation shader pattern
[shader("raygeneration")]
void RayGen() {
    RayDesc ray = computeRay(DispatchRaysIndex().xy);
    Payload payload;
    TraceRay(accelerationStructure, RAY_FLAG_NONE, 0xFF, 0, 0, 0, ray, payload);
    outputImage[DispatchRaysIndex().xy] = payload.color;
}
```

## Testing and Profiling

- Use GPU profiling tools (RenderDoc, Nsight) for performance analysis
- Implement frame time graphs and statistics
- Test on multiple GPU vendors (NVIDIA, AMD, Intel)
- Validate synchronization with VK_LAYER_KHRONOS_synchronization2
- Use automated screenshot comparison for visual regression testing
