use ash::vk;
use rust_vk::{
    render::{Render, RenderInitInfo},
    resource_type::buffer::RhiBuffer,
};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex
{
    pos: [f32; 4],
    color: [f32; 4],
}
const INDEX_DATA: [u32; 3] = [0u32, 1, 2];
const VERTEX_DATA: [Vertex; 3] = [
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
];

struct HelloRT
{
    vertex_buffer: Option<RhiBuffer>,
    index_buffer: Option<RhiBuffer>,
}


impl HelloRT
{
    fn to_vk_geo(&mut self)
    {
        let vertex_buffer = self.vertex_buffer.as_ref().unwrap();
        let index_buffer = self.index_buffer.as_ref().unwrap();


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

        let geometry = vk::AccelerationStructureGeometryKHR {
            geometry_type: vk::GeometryTypeKHR::TRIANGLES,
            flags: vk::GeometryFlagsKHR::OPAQUE,
            geometry: vk::AccelerationStructureGeometryDataKHR {
                triangles: triangles_data,
            },
            ..Default::default()
        };

        let range_info = vk::AccelerationStructureBuildRangeInfoKHR {
            first_vertex: 0,
            primitive_count: INDEX_DATA.len() as u32 / 3,
            primitive_offset: 0,
            transform_offset: 0,
        };

        // 构建 BLAS
        vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE;

        vk::AccelerationStructureKHR()
    }
}


fn main()
{
    Render::init(&RenderInitInfo {
        window_width: 800,
        window_height: 800,
        app_name: "hello-triangle".to_string(),
    });

    log::info!("start.");

    let mut index_buffer = RhiBuffer::new_index_buffer(std::mem::size_of_val(&INDEX_DATA), "index-buffer");
    index_buffer.transfer_data(&INDEX_DATA);

    let mut vertex_buffer = RhiBuffer::new_vertex_buffer(std::mem::size_of_val(&VERTEX_DATA), "vertex-buffer");
    vertex_buffer.transfer_data(&VERTEX_DATA);
}
