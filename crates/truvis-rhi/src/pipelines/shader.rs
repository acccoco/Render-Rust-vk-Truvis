use std::{ffi::CStr, rc::Rc};

use ash::vk;

use crate::foundation::{debug_messenger::DebugType, device::DeviceFunctions};

/// # Destroy
///
/// 需要手动调用 `destroy` 方法来释放资源。
pub struct ShaderModule
{
    handle: vk::ShaderModule,

    device_functions: Rc<DeviceFunctions>,
}
impl DebugType for ShaderModule
{
    fn debug_type_name() -> &'static str
    {
        "RhiShaderModule"
    }

    fn vk_handle(&self) -> impl vk::Handle
    {
        self.handle
    }
}
impl ShaderModule
{
    /// # param
    /// * path - spv shader 文件路径
    pub fn new(device_functions: Rc<DeviceFunctions>, path: &std::path::Path) -> Self
    {
        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);

        unsafe {
            let shader_module = device_functions.create_shader_module(&shader_module_info, None).unwrap();
            let shader_module = Self {
                handle: shader_module,
                device_functions: device_functions.clone(),
            };
            device_functions.set_debug_name(&shader_module, path.to_str().unwrap());
            shader_module
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule
    {
        self.handle
    }

    #[inline]
    pub fn destroy(self)
    {
        unsafe {
            self.device_functions.destroy_shader_module(self.handle, None);
        }
    }
}

#[derive(Clone)]
pub struct ShaderStageInfo
{
    pub stage: vk::ShaderStageFlags,
    pub entry_point: &'static CStr,
    pub path: String,
}
impl ShaderStageInfo
{
    #[inline]
    pub fn path(&self) -> &std::path::Path
    {
        std::path::Path::new(self.path.as_str())
    }
}

/// 在 pipeline create info 的 groups 中，每个 shader group 的 index
///
/// 每个 shader group 可以由多个 shader 组成，每个 shader group 都是独一无二的
pub struct ShaderGroupInfo
{
    pub ty: vk::RayTracingShaderGroupTypeKHR,
    pub general: u32,
    pub closest_hit: u32,
    pub any_hit: u32,
    pub intersection: u32,
}
impl ShaderGroupInfo
{
    pub const fn unused() -> Self
    {
        Self {
            ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
            general: vk::SHADER_UNUSED_KHR,
            closest_hit: vk::SHADER_UNUSED_KHR,
            any_hit: vk::SHADER_UNUSED_KHR,
            intersection: vk::SHADER_UNUSED_KHR,
        }
    }
}
