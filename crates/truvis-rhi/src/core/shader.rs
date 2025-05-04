use std::rc::Rc;

use ash::vk;

use crate::core::device::RhiDevice;

pub struct RhiShaderModule {
    pub handle: vk::ShaderModule,

    device: Rc<RhiDevice>,
}

impl RhiShaderModule {
    /// # param
    /// * path - spv shader 文件路径
    pub fn new(device: Rc<RhiDevice>, path: &std::path::Path) -> Self {
        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);

        unsafe {
            let shader_module = device.create_shader_module(&shader_module_info, None).unwrap();
            device.debug_utils.set_object_debug_name(shader_module, path.to_str().unwrap());
            Self {
                handle: shader_module,
                device,
            }
        }
    }

    pub fn destroy(self) {
        unsafe {
            self.device.destroy_shader_module(self.handle, None);
        }
    }
}
