#version 450

layout (location = 0) in vec4 pos;
layout (location = 1) in vec4 color;


layout (location = 0) out vec4 o_color;
layout (location = 1) out vec2 o_uv;

void main() {
    o_color = color;
    o_uv = pos.xy * 0.5 + 0.5;
    gl_Position = pos;
}
