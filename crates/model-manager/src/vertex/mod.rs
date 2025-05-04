use ash::vk;
pub mod vertex_3d;
pub mod vertex_pc;
pub mod vertex_pnu;

pub trait VertexLayout {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription>;

    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription>;
}
