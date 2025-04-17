struct PushConstants
{
    float3 camera_pos;
    int camera_pos_padding__;

    float3 camera_dir;
    int camera_dir_padding__;

    float2 mouse;
    float2 resolution;

    uint frame_id;
    float delta_time_ms;
    float time;
    float frame_rate;

    uint64_t scene_buffer_addr;
};

struct Light
{
    float3 pos;
    float pos_padding__;
    float3 color;
    float color_padding__;
};

struct SceneData
{
    float4x4 projection;
    float4x4 view;

    Light light_1;
    Light light_2;
    Light light_3;
};
