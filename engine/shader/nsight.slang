struct Instance
{
    uint geometry_indirect_idx;
    uint geometry_count;
    uint material_indirect_idx;
    uint material_count;

    float4x4 model;
    float4x4 inv_model;
};

struct TextureHandle
{
    int index;
};
struct ImageHandle
{
    int index;
};

struct Scene
{
    address64 all_mats;
    address64 all_geometries;

    address64 instance_material_map;
    address64 instance_geometry_map;

    address64 point_lights;
    address64 spot_lights;

    address64 all_instances;
    address64 tlas;

    uint point_light_count;
    uint spot_light_count;
    TextureHandle sky;
    TextureHandle uv_checker;
};

struct Geometry
{
    address64 position_buffer;
    address64 normal_buffer;
    address64 tangent_buffer;
    address64 uv_buffer;

    address64 index_buffer;
};

struct PBRMaterial
{
    float3 base_color;
    float metallic;

    float3 emissive;
    float roughness;

    TextureHandle diffuse_map;
    TextureHandle normal_map;
    float opaque;
    float _padding_1;
};

struct PerFrameData
{
    float4x4 projection;
    float4x4 view;
    float4x4 inv_view;
    float4x4 inv_projection;

    float3 camera_pos;
    float time_ms;

    float3 camera_forward;
    float delta_time_ms;

    float2 mouse_pos;
    float2 resolution;

    uint64 frame_id;
    /// 累计的帧数
    uint accum_frames;
    uint _padding_0;
};
