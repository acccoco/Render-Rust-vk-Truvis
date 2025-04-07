use std::marker::PhantomData;

use ash::vk;

use crate::{
    basic::DataUtils,
    core::{command_buffer::RhiCommandBuffer, descriptor::RhiDescriptorUpdateInfo},
    render_core::Rhi,
};

/// 游标类型
///
/// 用于描述游标类型，游标类型决定了描述符的类型。
pub enum ShaderCursorType
{
    Buffer,
    Image,
    Sampler,
}


pub struct Binding
{
    pub set: u32,
    pub binding: u32,
}

/// 某个 descriptor 的访问器
pub trait ShaderCursor
{
    fn get_binding(&self) -> Binding;

    fn get_type() -> vk::DescriptorType;

    fn write(&self, rhi: &Rhi, update_info: RhiDescriptorUpdateInfo);
}


pub struct BufferCursor<S: Sized>
{
    _phantom: PhantomData<S>,
}

impl<S: Sized> BufferCursor<S>
{
    fn write(cmd: &mut RhiCommandBuffer, buffer: vk::Buffer, data: &S)
    {
        cmd.cmd_update_buffer(buffer, size_of::<S>() as vk::DeviceSize, DataUtils::transform_u8(&data))
    }
}

pub struct Texture2DCursor
{
    set: u32,
    binding: u32,
}


impl Texture2DCursor
{
    pub fn new(set: u32, binding: u32) -> Self
    {
        Self { set, binding }
    }
}

impl ShaderCursor for Texture2DCursor
{
    fn get_binding(&self) -> Binding
    {
        todo!()
    }

    fn get_type() -> vk::DescriptorType
    {
        vk::DescriptorType::COMBINED_IMAGE_SAMPLER
    }

    fn write(&self, _rhi: &Rhi, _update_info: RhiDescriptorUpdateInfo)
    {
        todo!()
    }
}

/// material 这个 descriptor set 的定义
struct MaterialSet
{
    params: Texture2DCursor,
}
