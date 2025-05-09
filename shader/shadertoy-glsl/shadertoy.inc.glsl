layout(push_constant) uniform PushConstants {
    vec4 mouse;

    vec2 resolution;
    float time;
    float delta_time;

    int frame;
    float frame_rate;

    float __padding__1;
    float __padding__2;
} pc;