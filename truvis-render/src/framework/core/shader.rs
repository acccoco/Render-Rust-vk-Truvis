use std::rc::Rc;

use ash::vk;

use crate::framework::{core::device::RhiDevice, render_core::Rhi};

pub struct ShaderModule
{
    pub handle: vk::ShaderModule,

    device: Rc<RhiDevice>,
}

impl ShaderModule
{
    /// # param
    /// * path - spv shader 文件路径
    pub fn new(rhi: &Rhi, path: &std::path::Path) -> Self
    {
        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);

        unsafe {
            let shader_module = rhi.vk_device().create_shader_module(&shader_module_info, None).unwrap();
            rhi.set_debug_name(shader_module, path.to_str().unwrap());
            Self {
                handle: shader_module,
                device: rhi.device.clone(),
            }
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.device.destroy_shader_module(self.handle, None);
        }
    }
}
