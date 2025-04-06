mod data;

use std::rc::Rc;

use ash::vk;
use imgui::Ui;
use itertools::Itertools;
use truvis_render::{
    framework::{
        basic::{color::LabelColor, FRAME_ID_MAP},
        core::{
            buffer::{RhiBuffer, RhiBufferCreateInfo},
            command_queue::RhiSubmitInfo,
            descriptor::{DescriptorBindings, DescriptorSet, DescriptorSetLayout},
            pipeline::{Pipeline, PipelineTemplate},
        },
        render_core::Rhi,
        rendering::render_context::RenderContext,
    },
    render::{App, AppCtx, AppInitInfo, Renderer},
};

use crate::data::{ShapeBox, Vertex};

struct SceneDescriptorBinding;
impl DescriptorBindings for SceneDescriptorBinding
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
impl DescriptorBindings for MeshDescriptorBinding
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
impl DescriptorBindings for MaterialDescriptorBinding
{
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
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
    scene_layout: DescriptorSetLayout<SceneDescriptorBinding>,
    mesh_layout: DescriptorSetLayout<MeshDescriptorBinding>,
    material_layout: DescriptorSetLayout<MaterialDescriptorBinding>,
}

struct PhongAppDescriptorSets
{
    scene_set: DescriptorSet<SceneDescriptorBinding>,
    mesh_set: DescriptorSet<MeshDescriptorBinding>,
    material_set: DescriptorSet<MaterialDescriptorBinding>,
}


#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Light
{
    pos: glam::Vec3,
    pos_padding__: i32,

    color: glam::Vec3,
    color_padding__: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SceneUBO
{
    projection: glam::Mat4,
    view: glam::Mat4,

    l1: Light,
    l2: Light,
    l3: Light,
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
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct PushConstant
{
    camera_pos: glam::Vec3,
    camera_pos_padding__: i32,

    camera_dir: glam::Vec3,
    camera_dir_padding__: i32,

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
    pipeline: Pipeline,

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
        let scene_descriptor_set_layout = DescriptorSetLayout::<SceneDescriptorBinding>::new(rhi, "phong-scene");
        let mesh_descriptor_set_layout = DescriptorSetLayout::<MeshDescriptorBinding>::new(rhi, "phong-mesh");
        let material_descriptor_set_layout =
            DescriptorSetLayout::<MaterialDescriptorBinding>::new(rhi, "phong-material");

        let layouts = PhongAppDescriptorSetLayouts {
            scene_layout: scene_descriptor_set_layout,
            mesh_layout: mesh_descriptor_set_layout,
            material_layout: material_descriptor_set_layout,
        };

        let sets = (0..frames_in_flight)
            .map(|idx| FRAME_ID_MAP[idx])
            .map(|tag| PhongAppDescriptorSets {
                scene_set: DescriptorSet::<SceneDescriptorBinding>::new(
                    rhi,
                    &layouts.scene_layout,
                    &format!("phong-scene-{}", tag),
                ),
                mesh_set: DescriptorSet::<MeshDescriptorBinding>::new(
                    rhi,
                    &layouts.mesh_layout,
                    &format!("phong-mesh-{}", tag),
                ),
                material_set: DescriptorSet::<MaterialDescriptorBinding>::new(
                    rhi,
                    &layouts.material_layout,
                    &format!("phong-material-{}", tag),
                ),
            })
            .collect_vec();

        (layouts, sets)
    }

    fn create_pipeline(
        rhi: &Rhi,
        render_ctx: &mut RenderContext,
        descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    ) -> Pipeline
    {
        let extent = render_ctx.swapchain_extent();
        PipelineTemplate {
            fragment_shader_path: Some("shader/phong/phong.ps.hlsl.spv".into()),
            vertex_shader_path: Some("shader/phong/phong.vs.hlsl.spv".into()),

            descriptor_set_layouts,

            push_constant_ranges: vec![vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(size_of::<PushConstant>() as u32)],

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
                stride: size_of::<Vertex>() as u32,
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
        .create_pipeline(rhi, "phong")
    }

    fn create_vertices(rhi: &Rhi) -> RhiBuffer
    {
        let mut vertex_buffer =
            RhiBuffer::new_vertex_buffer(rhi, size_of_val(&ShapeBox::VERTICES), "[phong]vertex-buffer");
        vertex_buffer.transfer_data_by_stage_buffer(rhi, &ShapeBox::VERTICES);

        vertex_buffer
    }

    /// mesh ubo 数量：32 个
    /// material ubo 数量：32 个
    fn create_uniform_buffers(
        rhi: &Rhi,
        frames_in_flight: usize,
        mesh_ubo_align: &mut vk::DeviceSize,
        mat_ubo_align: &mut vk::DeviceSize,
    ) -> Vec<PhongUniformBuffer>
    {
        let mesh_instance_count = 32;
        let material_instance_count = 32;

        *mesh_ubo_align = rhi.ubo_offset_align(size_of::<MeshUBO>() as vk::DeviceSize);
        *mat_ubo_align = rhi.ubo_offset_align(size_of::<MaterialUBO>() as vk::DeviceSize);

        let scene_buffer_ci = Rc::new(RhiBufferCreateInfo::new(
            size_of::<SceneUBO>() as vk::DeviceSize * frames_in_flight as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        ));
        let mesh_buffer_ci = Rc::new(RhiBufferCreateInfo::new(
            *mesh_ubo_align * mesh_instance_count * frames_in_flight as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        ));
        let material_buffer_ci = Rc::new(RhiBufferCreateInfo::new(
            *mat_ubo_align * material_instance_count * frames_in_flight as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        ));

        let ubo_alloc_ci = Rc::new(vk_mem::AllocationCreateInfo {
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        });

        (0..frames_in_flight)
            .map(|idx| FRAME_ID_MAP[idx])
            .map(|tag| PhongUniformBuffer {
                scene_uniform_buffer: RhiBuffer::new(
                    rhi,
                    scene_buffer_ci.clone(),
                    ubo_alloc_ci.clone(),
                    None,
                    &format!("scene-ubo-{}", tag),
                ),
                mesh_uniform_buffer: RhiBuffer::new(
                    rhi,
                    mesh_buffer_ci.clone(),
                    ubo_alloc_ci.clone(),
                    Some(*mesh_ubo_align),
                    &format!("mesh-ubo-{}", tag),
                ),
                material_uniform_buffer: RhiBuffer::new(
                    rhi,
                    material_buffer_ci.clone(),
                    ubo_alloc_ci.clone(),
                    Some(*mat_ubo_align),
                    &format!("material-ubo-{}", tag),
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
            .buffer(scene_uniform_buffer.handle())
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
            .buffer(mesh_uniform_buffer.handle())
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
            .buffer(material_uniform_buffer.handle())
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
    fn update_ui(&mut self, ui: &mut Ui)
    {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }

    fn update(&mut self, app_ctx: &mut AppCtx)
    {
        app_ctx.rhi.debug_utils.begin_queue_label(
            app_ctx.rhi.graphics_queue.handle,
            "[main-pass]update",
            LabelColor::COLOR_PASS,
        );
        {
            self.update_scene_uniform(app_ctx.rhi, app_ctx.render_context.current_frame_index());
        }
        app_ctx.rhi.debug_utils.end_queue_label(app_ctx.rhi.graphics_queue.handle);
    }

    fn draw(&self, app_ctx: &mut AppCtx)
    {
        let frame_id = app_ctx.render_context.current_frame_index();

        let color_attach = <Self as App>::get_color_attachment(app_ctx.render_context.current_present_image_view());
        let depth_attach = <Self as App>::get_depth_attachment(app_ctx.render_context.depth_view.handle());
        let render_info = <Self as App>::get_render_info(
            vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent: app_ctx.render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach),
            &depth_attach,
        );

        let cmd = RenderContext::alloc_command_buffer(app_ctx.render_context, "[main-pass]render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[phong-pass]draw");
        {
            cmd.cmd_begin_rendering(&render_info);
            cmd.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
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
            cmd.cmd_bind_vertex_buffers(0, std::slice::from_ref(&self.vertex_buffer), &[0]);

            for (mesh_idx, _) in self.mesh_ubo.iter().enumerate() {
                cmd.bind_descriptor_sets(
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.pipeline_layout,
                    1,
                    &[self.descriptor_sets[frame_id].mesh_set.descriptor_set],
                    &[(self.mesh_ubo_offset_align * mesh_idx as u64) as u32],
                );
                cmd.cmd_draw(ShapeBox::VERTICES.len() as u32, 1, 0, 0);
            }

            cmd.end_rendering();
        }
        cmd.end();

        app_ctx.rhi.graphics_queue.submit(vec![RhiSubmitInfo::new(&[cmd])], None);
    }

    fn init(rhi: &Rhi, render_context: &mut RenderContext) -> Self
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
            Self::create_uniform_buffers(rhi, 3, &mut mesh_ubo_offset_align, &mut mat_ubo_offset_align);
        let vertex_buffer = Self::create_vertices(rhi);

        let rot =
            glam::Mat4::from_euler(glam::EulerRot::XYZ, 30f32.to_radians(), 40f32.to_radians(), 50f32.to_radians());
        let mesh_trans = [
            glam::Mat4::from_translation(glam::vec3(10.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 10.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, 15.0, 10.0)),
            glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.0)) * rot,
            glam::Mat4::from_translation(glam::vec3(0.0, -10.0, 0.0)) * rot,
        ];


        let camera_pos = glam::vec3(20.0, 0.0, 0.0);
        let camera_dir = glam::vec3(-1.0, 0.0, 0.0);

        // vulkan NDC 是 X-Right, Y-Up 的，framebuffer 的起点又是左上角，需要翻转 Y 轴
        let mut projection = glam::Mat4::perspective_infinite_rh(90f32.to_radians(), 1.0, 0.1);
        projection.y_axis.y *= -1.0;

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
                projection,
                view: glam::Mat4::look_to_rh(camera_pos, camera_dir, glam::vec3(0.0, 1.0, 0.0)),
                l1: Light {
                    pos: glam::vec3(-20.0, 40.0, 0.0),
                    color: glam::vec3(5.0, 6.0, 1.0) * 2.0,
                    ..Default::default()
                },
                l2: Light {
                    pos: glam::vec3(40.0, 40.0, -30.0),
                    color: glam::vec3(1.0, 6.0, 7.0) * 3.0,
                    ..Default::default()
                },
                l3: Light {
                    pos: glam::vec3(40.0, 40.0, 30.0),
                    color: glam::vec3(5.0, 1.0, 8.0) * 3.0,
                    ..Default::default()
                },
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
                ..Default::default()
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
    Renderer::<PhongApp>::run();
}
