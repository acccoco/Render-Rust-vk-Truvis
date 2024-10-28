use ash::vk;
use crate::framework::rhi::Rhi;

pub struct RhiShaderModule
{
    pub(crate) handle: vk::ShaderModule,
}

impl RhiShaderModule
{
    pub fn new(path: &std::path::Path) -> Self
    {
        let rhi = Rhi::instance();

        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);

        unsafe {
            let shader_module = rhi.device().create_shader_module(&shader_module_info, None).unwrap();
            Self { handle: shader_module }
        }
    }

    pub fn destroy(self)
    {
        unsafe {
            Rhi::instance().device().destroy_shader_module(self.handle, None);
        }
    }
}
