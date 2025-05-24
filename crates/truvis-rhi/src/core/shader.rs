use std::rc::Rc;

use ash::vk;

use crate::core::device::RhiDevice;

/// # Destroy
///
/// 需要手动调用 `destroy` 方法来释放资源。
pub struct RhiShaderModule {
    handle: vk::ShaderModule,

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
            device.debug_utils().set_object_debug_name(shader_module, path.to_str().unwrap());
            Self {
                handle: shader_module,
                device,
            }
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

pub struct RhiShaderStageInfo {
    pub stage: vk::ShaderStageFlags,
    pub entry_point: String,
    pub path: std::path::PathBuf,
}
