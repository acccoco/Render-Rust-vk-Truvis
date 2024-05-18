#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 color;


layout (location = 0) out vec4 o_color;
layout (location = 1) out vec2 o_uv;

void main() {
    o_color = color;
    o_uv = pos.xy * 0.5 + 0.5;
    gl_Position = pos;
}
