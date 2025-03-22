use std::marker::PhantomData;

use ash::vk;

use crate::framework::{
    basic::DataUtils,
    core::{command_buffer::CommandBuffer, descriptor::RhiDescriptorUpdateInfo},
    render_core::Core,
};

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

    fn write(&self, rhi: &Core, update_info: RhiDescriptorUpdateInfo);
}


pub struct BufferCursor<S: Sized>
{
    _phantom: PhantomData<S>,
}

impl<S: Sized> BufferCursor<S>
{
    fn write(cmd: &mut CommandBuffer, buffer: vk::Buffer, data: &S)
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

    fn write(&self, rhi: &Core, update_info: RhiDescriptorUpdateInfo)
    {
        todo!()
    }
}

/// material 这个 descriptor set 的定义
struct MaterialSet
{
    params: Texture2DCursor,
}
