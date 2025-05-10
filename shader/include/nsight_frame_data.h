struct SubMesh
{
    float4x4 model;
    float4x4 inv_model;

    uint mat_id;
    uint _padding_1;
    uint _padding_2;
    uint _padding_3;
};

struct InstanceData
{
    vec4u instance_count;
    SubMesh instances[256];
};


/// 单个点光源
struct PointLight
{
    float3 pos;
    float _pos_padding;

    float3 color;
    float _color_padding;
};

struct SpotLight
{
    float3 pos;
    float inner_angle;

    float3 color;
    float outer_angle;

    float3 dir;
    float _dir_padding;
};

/// 场景中所有的点光源
struct LightData
{
    /// 0: point light, 1: spot light
    vec4u light_count;
    PointLight lights[256];
    SpotLight spot_lights[256];
};

struct PBRMaterial
{
    float3 base_color;
    float metallic;

    float3 emissive;
    float roughness;

    uint diffuse_map;
    uint normal_map;
    uint _padding_1;
    uint _padding_2;
};

struct MatData
{
    vec4u mat_count;
    PBRMaterial materials[256];
};

struct FrameData
{
    float4x4 projection;
    float4x4 view;

    float3 camera_pos;
    float time_ms;

    float3 camera_forward;
    float delta_time_ms;

    float2 mouse_pos;
    float2 resolution;

    uint64 frame_id;
    uint64 _padding_1;

    LightData light_data;
    MatData mat_data;
    InstanceData ins_data;
};

FrameData frame_data;