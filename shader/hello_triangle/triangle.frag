#version 450

layout (location = 0) in vec4 o_color;
layout (location = 1) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = vec4(o_uv, 0.0, 1.0);
}
