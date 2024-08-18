#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#define SHADERTOY

layout(push_constant) uniform PushConstants {
    vec4 mouse;
    vec2 resolution;
    float time;
    float delta_time;
    int frame;
    float frame_rate;
} pc;

layout (location = 0) in vec2 fragCoord;

layout (location = 0) out vec4 fragColor;

#define iTime pc.time
#define iTimeDelta pc.delta_time
#define iResolution pc.resolution
#define iFrame pc.frame
#define iFrameRate pc.frame_rate
#define iMouse pc.mouse


// ---------------------------

#include "works/chainsaw_man_power.glsl"
