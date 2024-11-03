use ash::vk;

use crate::framework::rhi::Rhi;

pub struct RhiShaderModule<'a>
{
    pub(crate) handle: vk::ShaderModule,
    rhi: &'a Rhi,
}

impl<'a> RhiShaderModule<'a>
{
    pub fn new(rhi: &'a Rhi, path: &std::path::Path) -> Self
    {
        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);

        unsafe {
            let shader_module = rhi.device().create_shader_module(&shader_module_info, None).unwrap();
            Self {
                handle: shader_module,
                rhi,
            }
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            self.rhi.device().destroy_shader_module(self.handle, None);
        }
    }
}
