mod data;

use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use truvis_render::{
    framework::{
        basic::color::{BLUE, GREEN},
        core::{
            buffer::RhiBuffer,
            descriptor::{RhiDescriptorBindings, RhiDescriptorLayout, RhiDescriptorSet},
            pipeline::{RhiPipeline, RhiPipelineTemplate},
            queue::RhiSubmitInfo,
        },
        rendering::render_context::RenderContext,
        rhi::Rhi,
    },
    render::{AppInitInfo, Timer},
    run::{run, App},
};

use crate::data::{Vertex, BOX};

struct SceneDescriptorBinding;
impl RhiDescriptorBindings for SceneDescriptorBinding
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
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
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)]
    }
}

struct MaterialDescriptorBinding;
impl RhiDescriptorBindings for MaterialDescriptorBinding
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
            // vk::DescriptorSetLayoutBinding::default()
            //     .binding(1)
            //     .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            //     .descriptor_count(1)
            //     .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ]
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
#[derive(Clone, Copy)]
struct SceneUBO
{
    light_pos: glam::Vec3,
    light_color: glam::Vec3,
    projection: glam::Mat4,
    view: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MeshUBO
{
    model: glam::Mat4,
    trans_inv_model: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MaterialUBO
{
    color: glam::Vec4,
}


// TODO 考虑差分为 vertex 的和 fragment 的；
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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

struct PhongUniformBuffer
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

    /// BOX
    vertex_buffer: RhiBuffer,

    /// 每帧独立的 uniform buffer
    uniform_buffers: Vec<PhongUniformBuffer>,

    mesh_ubo_offset_align: vk::DeviceSize,
    mat_ubo_offset_align: vk::DeviceSize,

    mesh_ubo: Vec<MeshUBO>,
    mat_ubo: Vec<MaterialUBO>,
    scene_ubo: SceneUBO,

    push: PushConstant,
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

            push_constant_ranges: vec![vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(std::mem::size_of::<PushConstant>() as u32)],

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

    fn create_vertices(rhi: &'static Rhi) -> RhiBuffer
    {
        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(rhi, std::mem::size_of_val(&BOX), "vertex-buffer");
        vertex_buffer.transfer_data_by_stage_buffer(&BOX, "[phong-pass][init]transfer-vertex-data");

        vertex_buffer
    }

    /// mesh ubo 数量：32 个
    /// material ubo 数量：32 个
    fn create_uniform_buffers(
        rhi: &'static Rhi,
        render_ctx: &mut RenderContext,
        frames_in_flight: usize,
        mesh_ubo_align: &mut vk::DeviceSize,
        mat_ubo_align: &mut vk::DeviceSize,
    ) -> Vec<PhongUniformBuffer>
    {
        let mesh_instance_count = 32;
        let material_instance_count = 32;

        *mesh_ubo_align = rhi.ubo_offset_align(size_of::<MeshUBO>() as vk::DeviceSize);
        *mat_ubo_align = rhi.ubo_offset_align(size_of::<MaterialUBO>() as vk::DeviceSize);

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
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        (0..frames_in_flight)
            .map(|_| PhongUniformBuffer {
                scene_uniform_buffer: RhiBuffer::new2(rhi, &scene_buffer_ci, &ubo_alloc_ci, None, "scene-ubo"),
                mesh_uniform_buffer: RhiBuffer::new2(
                    rhi,
                    &mesh_buffer_ci,
                    &ubo_alloc_ci,
                    Some(*mesh_ubo_align),
                    "mesh-ubo",
                ),
                material_uniform_buffer: RhiBuffer::new2(
                    rhi,
                    &material_buffer_ci,
                    &ubo_alloc_ci,
                    Some(*mat_ubo_align),
                    "material-ubo",
                ),
            })
            .collect_vec()
    }

    /// 将场景的信息更新到 uniform buffer 中去，并且和 descriptor set 绑定起来
    fn update_scene_uniform(&mut self, rhi: &Rhi, frame_index: usize)
    {
        let PhongUniformBuffer {
            scene_uniform_buffer,
            mesh_uniform_buffer,
            material_uniform_buffer,
        } = &mut self.uniform_buffers[frame_index];

        scene_uniform_buffer.transfer_data_by_mem_map(std::slice::from_ref(&self.scene_ubo));
        mesh_uniform_buffer.transfer_data_by_mem_map(&self.mesh_ubo);
        material_uniform_buffer.transfer_data_by_mem_map(&self.mat_ubo);

        let PhongAppDescriptorSets {
            scene_set,
            mesh_set,
            material_set,
        } = &mut self.descriptor_sets[frame_index];

        let scene_ubo_info = vk::DescriptorBufferInfo::default()
            .buffer(scene_uniform_buffer.handle)
            .offset(0)
            .range(size_of::<SceneUBO>() as vk::DeviceSize);
        let scene_write = vk::WriteDescriptorSet::default()
            .dst_set(scene_set.descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(std::slice::from_ref(&scene_ubo_info));

        let mesh_ubo_info = vk::DescriptorBufferInfo::default()
            .buffer(mesh_uniform_buffer.handle)
            .offset(0)
            .range(size_of::<MeshUBO>() as vk::DeviceSize);
        let mesh_write = vk::WriteDescriptorSet::default()
            .dst_set(mesh_set.descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(std::slice::from_ref(&mesh_ubo_info));

        let mat_ubo_info = vk::DescriptorBufferInfo::default()
            .buffer(material_uniform_buffer.handle)
            .offset(0)
            .range(size_of::<MaterialUBO>() as vk::DeviceSize);
        let mat_write = vk::WriteDescriptorSet::default()
            .dst_set(material_set.descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(std::slice::from_ref(&mat_ubo_info));

        rhi.write_descriptor_sets(&[scene_write, mesh_write, mat_write]);
    }
}

impl App for PhongApp
{
    fn udpate_ui(&mut self, ui: &mut Ui)
    {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        rhi.graphics_queue_begin_label("[main-pass]update", GREEN);
        {
            let frame_id = render_context.current_frame_index();
            self.update_scene_uniform(rhi, render_context.current_frame_index());
        }
        rhi.graphics_queue_end_label();
    }

    fn draw(&self, rhi: &'static Rhi, render_context: &mut RenderContext, timer: &Timer)
    {
        let frame_id = render_context.current_frame_index();

        let color_attach = <Self as App>::get_color_attachment(render_context.current_present_image_view());
        let depth_attach = <Self as App>::get_depth_attachment(render_context.depth_image_view);
        let render_info = <Self as App>::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );

        rhi.graphics_queue_begin_label("[main-pass]draw", GREEN);

        let mut cmd = RenderContext::alloc_command_buffer(render_context, "[main-pass]render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        cmd.begin_label("phong-pass-draw", BLUE);
        {
            cmd.cmd_begin_rendering(&render_info);
            cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
            cmd.cmd_push_constants(
                self.pipeline.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::bytes_of(&self.push),
            );

            // scene data
            cmd.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout,
                0,
                &[self.descriptor_sets[frame_id].scene_set.descriptor_set],
                &[0],
            );

            // per mat
            cmd.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout,
                2,
                &[self.descriptor_sets[frame_id].material_set.descriptor_set],
                // TODO 只使用一个材质
                &[0],
            );

            // index 和 vertex 暂且就用同一个
            cmd.bind_vertex_buffer(0, std::slice::from_ref(&self.vertex_buffer), &[0]);

            for (mesh_idx, mesh_ubo) in self.mesh_ubo.iter().enumerate() {
                cmd.bind_descriptor_sets(
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.pipeline_layout,
                    1,
                    &[self.descriptor_sets[frame_id].mesh_set.descriptor_set],
                    &[(self.mesh_ubo_offset_align * mesh_idx as u64) as u32],
                );
                cmd.draw((BOX.len() / 8) as u32, 1, 0, 0);
            }

            cmd.end_rendering();
        }
        cmd.end_label();
        cmd.end();

        rhi.graphics_queue_submit(
            vec![RhiSubmitInfo {
                command_buffers: vec![cmd],
                ..Default::default()
            }],
            None,
        );

        rhi.graphics_queue_end_label();
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

        let mut mesh_ubo_offset_align = 0;
        let mut mat_ubo_offset_align = 0;
        let uniform_buffers =
            Self::create_uniform_buffers(rhi, render_context, 3, &mut mesh_ubo_offset_align, &mut mat_ubo_offset_align);
        let vertex_buffer = Self::create_vertices(rhi);

        let mesh_trans = [
            glam::Mat4::from_translation(glam::vec3(10f32, 0f32, 0f32)),
            glam::Mat4::from_translation(glam::vec3(0f32, 10f32, 0f32)),
            glam::Mat4::from_translation(glam::vec3(0f32, 10f32, 10f32)),
        ];

        let camera_pos = glam::vec3(10f32, 10f32, 10f32);
        let camera_dir = glam::vec3(-1f32, -1f32, -1f32);

        Self {
            descriptor_set_layouts: layouts,
            descriptor_sets: sets,
            pipeline,
            vertex_buffer,
            uniform_buffers,
            mesh_ubo_offset_align,
            mat_ubo_offset_align,
            mesh_ubo: mesh_trans
                .iter()
                .map(|trans| MeshUBO {
                    model: *trans,
                    trans_inv_model: trans.inverse().transpose(),
                })
                .collect_vec(),
            mat_ubo: vec![MaterialUBO {
                color: glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
            }],
            scene_ubo: SceneUBO {
                light_pos: glam::vec3(20f32, 40f32, -20f32),
                light_color: glam::vec3(1f32, 1f32, 1f32),
                projection: glam::Mat4::perspective_infinite_rh(90f32.to_radians(), 1f32, 0.1f32),
                view: glam::Mat4::look_to_rh(camera_pos, camera_dir, glam::vec3(0.0, 1.0, 0.0)),
            },
            push: PushConstant {
                camera_pos,
                camera_dir,
                frame_id: 0,
                delta_time_ms: 0.0,
                mouse_pos: Default::default(),
                resolution: Default::default(),
                time_ms: 0.0,
                fps: 0.0,
            },
        }
    }

    fn get_render_init_info() -> AppInitInfo
    {
        AppInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "Phong".to_string(),
            enable_validation: true,
        }
    }
}


fn main()
{
    run::<PhongApp>()
}
