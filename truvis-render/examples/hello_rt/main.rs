use std::mem::offset_of;

use ash::vk;
use imgui::Ui;
use truvis_render::{
    framework::{
        core::{
            acceleration::Acceleration,
            buffer::Buffer,
            descriptor::DescriptorBindings,
            pipeline::{Pipeline, PipelineTemplate},
            queue::SubmitInfo,
        },
        rendering::render_context::RenderContext,
        render_core::Core,
    },
    render::{App, AppCtx, AppInitInfo, Renderer, Timer},
};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex
{
    pos: [f32; 4],
    color: [f32; 4],
}
const INDEX_DATA: [u32; 21] = [
    0u32, 1, 2, //
    0, 2, 3, //
    0, 1, 3, //
    1, 2, 3, //
    0, 3, 2, 0, 3, 1, 1, 3, 2,
];
const VERTEX_DATA: [Vertex; 4] = [
    Vertex {
        pos: [-1.0, 1.0, 0.0, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        pos: [1.0, 1.0, 0.0, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        pos: [0.0, -1.0, 0.0, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        pos: [0.0, 0.0, 1.0, 1.0],
        color: [1.0, 1.0, 1.0, 1.0],
    },
];


pub struct RayTracingBindings;
impl DescriptorBindings for RayTracingBindings
{
    // FIXME
    fn bindings() -> Vec<vk::DescriptorSetLayoutBinding<'static>>
    {
        vec![
            vk::DescriptorSetLayoutBinding {
                // TLAS
                binding: 0,
                descriptor_type: vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                // Output image
                binding: 0,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                ..Default::default()
            },
        ]
    }
}


struct HelloRT
{
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    pipeline: Pipeline,
    blas: Acceleration, // 可以有多个
    tlas: Acceleration, // 只能由一个
}


impl HelloRT
{
    fn init_buffer(rhi: &'static Core) -> (Buffer, Buffer)
    {
        let mut index_buffer = Buffer::new_index_buffer(rhi, size_of_val(&INDEX_DATA), "index-buffer");
        index_buffer.transfer_data_by_stage_buffer(&INDEX_DATA);

        let mut vertex_buffer = Buffer::new_vertex_buffer(rhi, size_of_val(&VERTEX_DATA), "vertex-buffer");
        vertex_buffer.transfer_data_by_stage_buffer(&VERTEX_DATA);

        (vertex_buffer, index_buffer)
    }

    fn init_acceleration(
        rhi: &'static Core,
        vertex_buffer: &Buffer,
        index_buffer: &Buffer,
    ) -> (Acceleration, Acceleration)
    {
        let triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR {
            vertex_format: vk::Format::R32G32B32_SFLOAT,
            vertex_data: vk::DeviceOrHostAddressConstKHR {
                device_address: vertex_buffer.get_device_address(),
            },
            vertex_stride: size_of::<Vertex>() as u64,
            max_vertex: VERTEX_DATA.len() as u32,

            index_type: vk::IndexType::UINT32,
            index_data: vk::DeviceOrHostAddressConstKHR {
                device_address: index_buffer.get_device_address(),
            },

            ..Default::default()
        };

        // 构建 BLAS
        let blas = Acceleration::build_blas(
            rhi,
            vec![(triangles_data, INDEX_DATA.len() as u32 / 3)],
            vk::BuildAccelerationStructureFlagsKHR::empty(),
            "hello",
        );


        // 3x4 row-major 的变换矩阵
        let trans = vk::TransformMatrixKHR {
            matrix: [
                1.0, 0.0, 0.0, 0.0, // row0
                0.0, 1.0, 0.0, 0.0, // row1
                0.0, 0.0, 1.0, 0.0, // row2
            ],
        };
        // 构建 TLAS
        // TODO 再确认一下每一个字段
        let instances = vec![vk::AccelerationStructureInstanceKHR {
            transform: trans,
            // only be hit if (rayMask & instance.mask != 0)
            instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0,
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as u8,
            ),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: blas.get_device_address(),
            },
        }];

        let tlas =
            Acceleration::build_tlas(rhi, &instances, vk::BuildAccelerationStructureFlagsKHR::empty(), "hello");


        (tlas, blas)
    }

    fn init_pipeline(rhi: &'static Core, render_context: &RenderContext) -> Pipeline
    {
        let extent = render_context.swapchain_extent();
        PipelineTemplate {
            fragment_shader_path: Some("shader/hello_triangle/triangle.frag.spv".into()),
            vertex_shader_path: Some("shader/hello_triangle/triangle.vert.spv".into()),
            color_formats: vec![render_context.color_format()],
            depth_format: render_context.depth_format(),
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
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Vertex, pos) as u32,
                },
                vk::VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: vk::Format::R32G32B32A32_SFLOAT,
                    offset: offset_of!(Vertex, color) as u32,
                },
            ],
            color_attach_blend_states: vec![vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)],
            ..Default::default()
        }
        .create_pipeline(rhi, "rt")
    }

    fn run(&self, rhi: &'static Core, render_context: &mut RenderContext)
    {
        let depth_attach_info = <Self as App>::get_depth_attachment(render_context.depth_image_view);
        let color_attach_info = <Self as App>::get_color_attachment(render_context.current_present_image_view());
        let render_info = <Self as App>::get_render_info(
            vk::Rect2D {
                offset: Default::default(),
                extent: render_context.swapchain_extent(),
            },
            std::slice::from_ref(&color_attach_info),
            &depth_attach_info,
        );

        let mut cmd = render_context.alloc_command_buffer("render");
        cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, "[main-pass]draw");
        {
            cmd.cmd_begin_rendering(&render_info);
            cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);
            cmd.bind_index_buffer(&self.index_buffer, 0, vk::IndexType::UINT32);
            cmd.bind_vertex_buffer(0, std::slice::from_ref(&self.vertex_buffer), &[0]);
            cmd.draw_indexed((INDEX_DATA.len() as u32, 0), (1, 0), 0);
            cmd.end_rendering();
        }
        cmd.end();
        rhi.graphics_queue().submit(
            rhi,
            vec![SubmitInfo {
                command_buffers: vec![cmd],
                ..Default::default()
            }],
            None,
        );
    }


    fn new(rhi: &'static Core, render_context: &RenderContext) -> Self
    {
        log::info!("start.");
        let (vertex_buffer, index_buffer) = Self::init_buffer(rhi);
        let (tlas, blas) = Self::init_acceleration(rhi, &vertex_buffer, &index_buffer);
        let pipeline = Self::init_pipeline(rhi, render_context);

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            blas,
            tlas,
        }
    }
}

impl App for HelloRT
{
    fn update(&mut self, app_ctx: &mut AppCtx)
    {
        //
    }

    fn draw(&self, app_ctx: &mut AppCtx)
    {
        self.run(app_ctx.rhi, app_ctx.render_context);
    }

    fn init(rhi: &'static Core, render_context: &mut RenderContext) -> Self
    {
        HelloRT::new(rhi, render_context)
    }

    fn get_render_init_info() -> AppInitInfo
    {
        AppInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "hello-triangle".to_string(),
            enable_validation: true,
        }
    }

    fn update_ui(&mut self, ui: &mut Ui)
    {
        ui.text_wrapped("Hello world!");
        ui.text_wrapped("こんにちは世界！");
    }
}


fn main()
{
    Renderer::<HelloRT>::run();
}
