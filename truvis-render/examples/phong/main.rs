mod data;

use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use truvis_render::{
    framework::{
        core::{
            buffer::RhiBuffer,
            descriptor::{RhiDescriptorBindings, RhiDescriptorLayout, RhiDescriptorSet},
            pipeline::{RhiPipeline, RhiPipelineTemplate},
        },
        rendering::render_context::RenderContext,
        rhi::Rhi,
    },
    render::{AppInitInfo, Timer},
    run::App,
};

use crate::data::Vertex;

struct SceneDescriptorBinding;
impl RhiDescriptorBindings for SceneDescriptorBinding
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)]
    }
}

struct MeshDescriptorBinding;
impl RhiDescriptorBindings for MeshDescriptorBinding
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)]
    }
}

struct MaterialDescriptorBinding;
impl RhiDescriptorBindings for MaterialDescriptorBinding
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)]
    }
}


struct PhongAppDescriptorSetLayouts
{
    scene_layout: RhiDescriptorLayout<SceneDescriptorBinding>,
    mesh_layout: RhiDescriptorLayout<MeshDescriptorBinding>,
    material_layout: RhiDescriptorLayout<MaterialDescriptorBinding>,
}

struct PhongAppDescriptorSets
{
    scene_set: RhiDescriptorSet<SceneDescriptorBinding>,
    mesh_set: RhiDescriptorSet<MeshDescriptorBinding>,
    material_set: RhiDescriptorSet<MaterialDescriptorBinding>,
}

#[repr(C)]
struct SceneUBO
{
    light_pos: glam::Vec3,
    light_color: glam::Vec3,
    projection: glam::Mat4,
    view: glam::Mat4,
}

#[repr(C)]
struct MeshUBO
{
    model: glam::Mat4,
    trans_inv_model: glam::Mat4,
}

#[repr(C)]
struct MaterialUBO
{
    color: glam::Vec4,
}

#[repr(C)]
struct PushConstant
{
    camera_pos: glam::Vec3,
    camera_dir: glam::Vec3,
    frame_id: u32,
    delta_time_ms: f32,
    mouse_pos: glam::Vec2,
    resolution: glam::Vec2,
    time_ms: f32,
    fps: f32,
}

struct PhongBuffer
{
    scene_uniform_buffer: RhiBuffer,
    mesh_uniform_buffer: RhiBuffer,
    material_uniform_buffer: RhiBuffer,
}

struct PhongApp
{
    descriptor_set_layouts: PhongAppDescriptorSetLayouts,

    /// 每帧独立的 descriptor set
    descriptor_sets: Vec<PhongAppDescriptorSets>,
    pipeline: RhiPipeline,

    /// 每帧独立的 uniform buffer
    mesh_uniform_buffers: Vec<PhongBuffer>,

    mesh_ubo_align: vk::DeviceSize,
    scene_ubo_align: vk::DeviceSize,
}


impl PhongApp
{
    fn create_descriptor_sets(
        rhi: &Rhi,
        frames_in_flight: usize,
    ) -> (PhongAppDescriptorSetLayouts, Vec<PhongAppDescriptorSets>)
    {
        let scene_descriptor_set_layout = RhiDescriptorLayout::<SceneDescriptorBinding>::new(rhi);
        let mesh_descriptor_set_layout = RhiDescriptorLayout::<MeshDescriptorBinding>::new(rhi);
        let material_descriptor_set_layout = RhiDescriptorLayout::<MaterialDescriptorBinding>::new(rhi);

        let layouts = PhongAppDescriptorSetLayouts {
            scene_layout: scene_descriptor_set_layout,
            mesh_layout: mesh_descriptor_set_layout,
            material_layout: material_descriptor_set_layout,
        };

        let sets = (0..frames_in_flight)
            .map(|_| PhongAppDescriptorSets {
                scene_set: RhiDescriptorSet::<SceneDescriptorBinding>::new(rhi, &layouts.scene_layout),
                mesh_set: RhiDescriptorSet::<MeshDescriptorBinding>::new(rhi, &layouts.mesh_layout),
                material_set: RhiDescriptorSet::<MaterialDescriptorBinding>::new(rhi, &layouts.material_layout),
            })
            .collect_vec();

        (layouts, sets)
    }

    fn create_pipeline(
        rhi: &'static Rhi,
        render_ctx: &mut RenderContext,
        descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    ) -> RhiPipeline
    {
        let extent = render_ctx.swapchain_extent();
        let pipeline = RhiPipelineTemplate {
            fragment_shader_path: Some("shader/phong/phong.ps.hlsl.spv".into()),
            vertex_shader_path: Some("shader/phong/phong.vs.hlsl.spv".into()),

            descriptor_set_layouts,

            color_formats: vec![render_ctx.color_format()],
            depth_format: render_ctx.depth_format(),
            viewport: Some(vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as _,
                height: extent.height as _,
                min_depth: 0.0,
                max_depth: 1.0,
            }),
            scissor: Some(extent.into()),
            vertex_binding_desc: vec![vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }],
            vertex_attribute_desec: vec![
                vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: std::mem::offset_of!(Vertex, pos) as u32,
                },
                vk::VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: std::mem::offset_of!(Vertex, normal) as u32,
                },
                vk::VertexInputAttributeDescription {
                    location: 2,
                    binding: 0,
                    format: vk::Format::R32G32_SFLOAT,
                    offset: std::mem::offset_of!(Vertex, uv) as u32,
                },
            ],
            color_attach_blend_states: vec![vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)],
            ..Default::default()
        }
        .create_pipeline(rhi, "");

        pipeline
    }


    /// mesh ubo 数量：32 个
    /// material ubo 数量：32 个
    fn create_buffers(
        rhi: &'static Rhi,
        render_ctx: &mut RenderContext,
        frames_in_flight: usize,
        mesh_ubo_align: &mut vk::DeviceSize,
        mat_ubo_align: &mut vk::DeviceSize,
    ) -> Vec<PhongBuffer>
    {
        let mesh_instance_count = 32;
        let material_instance_count = 32;

        *mesh_ubo_align = rhi.ubo_align(size_of::<MeshUBO>() as vk::DeviceSize);
        *mat_ubo_align = rhi.ubo_align(size_of::<MaterialUBO>() as vk::DeviceSize);

        let scene_buffer_ci = vk::BufferCreateInfo::default()
            .size(size_of::<SceneUBO>() as vk::DeviceSize * frames_in_flight as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER);
        let mesh_buffer_ci = vk::BufferCreateInfo::default()
            .size(*mesh_ubo_align * mesh_instance_count * frames_in_flight as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER);
        let material_buffer_ci = vk::BufferCreateInfo::default()
            .size(*mat_ubo_align * material_instance_count * frames_in_flight as u64)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER);

        let ubo_alloc_ci = vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE,
            ..Default::default()
        };

        (0..frames_in_flight)
            .map(|_| PhongBuffer {
                scene_uniform_buffer: RhiBuffer::new2(rhi, &scene_buffer_ci, &ubo_alloc_ci, None, "scene-ubo"),
                mesh_uniform_buffer: RhiBuffer::new2(
                    rhi,
                    &mesh_buffer_ci,
                    &ubo_alloc_ci,
                    Some(mesh_ubo_align),
                    "mesh-ubo",
                ),
                material_uniform_buffer: RhiBuffer::new2(
                    rhi,
                    &material_buffer_ci,
                    &ubo_alloc_ci,
                    Some(material_ubo_align),
                    "material-ubo",
                ),
            })
            .collect_vec()
    }

    // FIXME 不同信息的更新方式是不一样的：
    //  mesh 可以通过 push constant 去；效率最高
    //  mat 需要将 buffer 更新到 descriptor set 中去
    //  scene 的数据不需要重新绑定 buffer，而是需要将数据更新到 buffer 中去
    /// 将场景的信息更新到 uniform buffer 中去
    fn update_scene_uniform(&mut self, rhi: &Rhi, frame_index: usize)
    {
        todo!()
    }
}

impl App for PhongApp
{
    fn update(&self, ui: &mut Ui)
    {
        todo!()
    }

    fn draw(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        todo!()
    }

    fn init(rhi: &'static Rhi, render_context: &mut RenderContext) -> Self
    {
        // TODO 通过其他方式获取到 frames in flight
        let (layouts, sets) = Self::create_descriptor_sets(rhi, 3);
        let pipeline = Self::create_pipeline(
            rhi,
            render_context,
            vec![
                layouts.scene_layout.layout,
                layouts.mesh_layout.layout,
                layouts.material_layout.layout,
            ],
        );

        Self {
            descriptor_set_layouts: layouts,
            descriptor_sets: sets,
            pipeline,
        }
    }

    fn get_render_init_info() -> AppInitInfo
    {
        todo!()
    }
}


fn main() {}
