#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout(push_constant) uniform PushConstants {
    float time;
    float delta_time;
    int frame;
    float frame_rate;
    vec2 resolution;
} pc;

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 color;


layout (location = 0) out vec2 fragCoord;

void main() {
    // NDC 坐标系是 left-top(-1, -1), right-bottom(1, 1)
    // shadertoy 坐标系是 left-bottom(0, 0), right-top(1, 1)
    fragCoord = (pos.xy * vec2(0.5, -0.5) + 0.5) * pc.resolution;
    gl_Position = pos;
}
