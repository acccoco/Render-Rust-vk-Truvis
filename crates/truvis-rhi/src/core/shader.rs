use std::ffi::CStr;
use std::rc::Rc;

use crate::core::debug_utils::RhiDebugType;
use crate::core::device::RhiDevice;
use ash::vk;

/// # Destroy
///
/// 需要手动调用 `destroy` 方法来释放资源。
pub struct RhiShaderModule {
    handle: vk::ShaderModule,

    device: Rc<RhiDevice>,
}
impl RhiDebugType for RhiShaderModule {
    fn debug_type_name() -> &'static str {
        "RhiShaderModule"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
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
            let shader_module = Self {
                handle: shader_module,
                device: device.clone(),
            };
            device.debug_utils().set_debug_name(&shader_module, path.to_str().unwrap());
            shader_module
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.handle
    }

    #[inline]
    pub fn destroy(self) {
        unsafe {
            self.device.destroy_shader_module(self.handle, None);
        }
    }
}

#[derive(Copy, Clone)]
pub struct RhiShaderStageInfo {
    pub stage: vk::ShaderStageFlags,
    pub entry_point: &'static CStr,
    pub path: &'static str,
}
impl RhiShaderStageInfo {
    #[inline]
    pub fn path(&self) -> &std::path::Path {
        std::path::Path::new(self.path)
    }
}

/// 在 pipeline create info 的 groups 中，每个 shader group 的 index
///
/// 每个 shader group 可以由多个 shader 组成，每个 shader group 都是独一无二的
pub struct ShaderGroupInfo {
    pub ty: vk::RayTracingShaderGroupTypeKHR,
    pub general: u32,
    pub closest_hit: u32,
    pub any_hit: u32,
    pub intersection: u32,
}
impl ShaderGroupInfo {
    pub const fn unused() -> Self {
        Self {
            ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
            general: vk::SHADER_UNUSED_KHR,
            closest_hit: vk::SHADER_UNUSED_KHR,
            any_hit: vk::SHADER_UNUSED_KHR,
            intersection: vk::SHADER_UNUSED_KHR,
        }
    }
}
