use ash::vk;

use crate::framework::render_core::Core;

pub struct ShaderModule
{
    pub(crate) handle: vk::ShaderModule,
    rhi: &'static Core,
}

impl ShaderModule
{
    pub fn new(rhi: &'static Core, path: &std::path::Path) -> Self
    {
        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);

        unsafe {
            let shader_module = rhi.vk_device().create_shader_module(&shader_module_info, None).unwrap();
            rhi.set_debug_name(shader_module, path.to_str().unwrap());
            Self {
                handle: shader_module,
                rhi,
            }
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.rhi.vk_device().destroy_shader_module(self.handle, None);
        }
    }
}
