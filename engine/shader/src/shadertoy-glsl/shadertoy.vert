#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "shadertoy.inc.glsl"


layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec3 tangent;
layout (location = 3) in vec2 uv;

layout (location = 0) out vec2 fragCoord;

void main() {
    // NDC 坐标系是 left-top(-1, -1), right-bottom(1, 1)
    // shadertoy 坐标系是 left-bottom(0, 0), right-top(1, 1)
    fragCoord = (pos.xy * vec2(0.5, -0.5) + 0.5) * pc.resolution;
    gl_Position = vec4(pos, 1.0);
}
