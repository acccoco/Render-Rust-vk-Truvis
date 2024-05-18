#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec4 o_color;
layout (location = 1) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = vec4(o_uv, 0.0, 1.0);
}
