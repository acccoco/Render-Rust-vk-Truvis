use ash::vk;
use memoffset::offset_of;
use truvis_render::{
    render::{RenderInitInfo, Renderer},
    render_context::RenderContext,
    rhi::Rhi,
    rhi_type::{
        acceleration::RhiAcceleration,
        buffer::RhiBuffer,
        pipeline::{RhiPipeline, RhiPipelineTemplate},
        queue::RhiSubmitBatch,
    },
    window_system::WindowSystem,
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

struct HelloRT
{
    vertex_buffer: RhiBuffer,
    index_buffer: RhiBuffer,
    pipeline: RhiPipeline,
    blas: RhiAcceleration, // 可以有多个
    tlas: RhiAcceleration, // 只能由一个
}


impl HelloRT
{
    fn init_buffer() -> (RhiBuffer, RhiBuffer)
    {
        let mut index_buffer = RhiBuffer::new_index_buffer(std::mem::size_of_val(&INDEX_DATA), "index-buffer");
        index_buffer.transfer_data(&INDEX_DATA);

        let mut vertex_buffer = RhiBuffer::new_vertex_buffer(std::mem::size_of_val(&VERTEX_DATA), "vertex-buffer");
        vertex_buffer.transfer_data(&VERTEX_DATA);

        (vertex_buffer, index_buffer)
    }

    fn init_acceleration(vertex_buffer: &RhiBuffer, index_buffer: &RhiBuffer) -> (RhiAcceleration, RhiAcceleration)
    {
        let triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR {
            vertex_format: vk::Format::R32G32B32_SFLOAT,
            vertex_data: vk::DeviceOrHostAddressConstKHR {
                device_address: vertex_buffer.get_device_address(),
            },
            vertex_stride: std::mem::size_of::<Vertex>() as u64,
            max_vertex: VERTEX_DATA.len() as u32,

            index_type: vk::IndexType::UINT32,
            index_data: vk::DeviceOrHostAddressConstKHR {
                device_address: index_buffer.get_device_address(),
            },

            ..Default::default()
        };

        // 构建 BLAS
        let blas = RhiAcceleration::build_blas(
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

        let tlas = RhiAcceleration::build_tlas(&instances, vk::BuildAccelerationStructureFlagsKHR::empty(), "hello");


        (tlas, blas)
    }

    fn init_pipeline() -> RhiPipeline
    {
        let extent = RenderContext::extent();
        let pipeline = RhiPipelineTemplate {
            fragment_shader_path: Some("shader/hello_triangle/triangle.frag.spv".into()),
            vertex_shader_path: Some("shader/hello_triangle/triangle.vert.spv".into()),
            color_formats: vec![RenderContext::instance().color_format()],
            depth_format: RenderContext::depth_format(),
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
            color_attach_blend_states: vec![vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .build()],
            ..Default::default()
        }
        .create_pipeline("");

        pipeline
    }

    fn run(&self)
    {
        WindowSystem::instance().render_loop(|| {
            RenderContext::acquire_frame();

            let rhi = Rhi::instance();

            let mut cmd = RenderContext::alloc_command_buffer("render");
            cmd.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            {
                cmd.begin_rendering(&RenderContext::render_info());
                cmd.bind_pipeline(vk::PipelineBindPoint::GRAPHICS, &self.pipeline);
                cmd.bind_index_buffer(&self.index_buffer, 0, vk::IndexType::UINT32);
                cmd.bind_vertex_buffer(0, std::slice::from_ref(&self.vertex_buffer), &[0]);
                cmd.draw_indexed((INDEX_DATA.len() as u32, 0), (1, 0), 0);
                cmd.end_rendering();
            }
            cmd.end();
            rhi.graphics_queue().submit(
                vec![RhiSubmitBatch {
                    command_buffers: vec![cmd],
                    ..Default::default()
                }],
                None,
            );

            RenderContext::submit_frame();
        });
    }

    fn init() -> Self
    {
        Renderer::init(&RenderInitInfo {
            window_width: 800,
            window_height: 800,
            app_name: "hello-triangle".to_string(),
        })
        .expect("init failed");

        log::info!("start.");

        let (vertex_buffer, index_buffer) = Self::init_buffer();
        let (tlas, blas) = Self::init_acceleration(&vertex_buffer, &index_buffer);
        let pipeline = Self::init_pipeline();

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            blas,
            tlas,
        }
    }

    fn create_descriptor_set()
    {
        // vk::DescriptorSetLayoutBinding{}
    }
}


fn main()
{
    let hello = HelloRT::init();
    hello.run();
}
