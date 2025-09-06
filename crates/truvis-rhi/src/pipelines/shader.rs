use std::collections::HashMap;
use std::ffi::CStr;

use ash::vk;

use crate::{foundation::debug_messenger::DebugType, render_context::RenderContext};

/// # Destroy
///
/// 需要手动调用 `destroy` 方法来释放资源。
pub struct ShaderModule {
    handle: vk::ShaderModule,

    #[cfg(debug_assertions)]
    destroyed: bool,
}
impl ShaderModule {
    /// # param
    /// * path - spv shader 文件路径
    pub fn new(path: &std::path::Path) -> Self {
        let device_functions = RenderContext::get().device_functions();
        let mut file = std::fs::File::open(path).unwrap();
        let shader_code = ash::util::read_spv(&mut file).unwrap();

        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);

        unsafe {
            let shader_module = device_functions.create_shader_module(&shader_module_info, None).unwrap();
            let shader_module = Self {
                handle: shader_module,

                #[cfg(debug_assertions)]
                destroyed: false,
            };
            device_functions.set_debug_name(&shader_module, path.to_str().unwrap());
            shader_module
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.handle
    }

    #[inline]
    pub fn destroy(mut self) {
        let device_functions = RenderContext::get().device_functions();
        unsafe {
            device_functions.destroy_shader_module(self.handle, None);
        }
        #[cfg(debug_assertions)]
        {
            self.destroyed = true;
        }
    }
}
impl Drop for ShaderModule {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        debug_assert!(self.destroyed, "ShaderModule must be destroyed manually before drop.");
    }
}
impl DebugType for ShaderModule {
    fn debug_type_name() -> &'static str {
        "RhiShaderModule"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}

/// 可以存放多个 ShaderModule，使用路径进行索引
pub struct ShaderModuleCache {
    shader_modules: HashMap<String, ShaderModule>,
    #[cfg(debug_assertions)]
    destroyed: bool,
}
impl ShaderModuleCache {
    pub fn new() -> Self {
        Self {
            shader_modules: HashMap::new(),
            #[cfg(debug_assertions)]
            destroyed: false,
        }
    }

    pub fn get_or_load(&mut self, path: &std::path::Path) -> &ShaderModule {
        let path_str = path.to_str().unwrap().to_string();
        self.shader_modules.entry(path_str).or_insert_with(|| ShaderModule::new(path))
    }

    pub fn destroy(mut self) {
        #[cfg(debug_assertions)]
        {
            self.destroyed = true;
        }

        // 使用 std::mem::take 来 move 出 HashMap，留下一个空的 HashMap
        let shader_modules = std::mem::take(&mut self.shader_modules);
        shader_modules.into_values().for_each(|module| module.destroy());
    }
}
impl Drop for ShaderModuleCache {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        debug_assert!(self.destroyed, "ShaderModuleCache must be destroyed manually before drop.");
    }
}

#[derive(Clone)]
pub struct ShaderStageInfo {
    pub stage: vk::ShaderStageFlags,
    pub entry_point: &'static CStr,
    pub path: String,
}
impl ShaderStageInfo {
    #[inline]
    pub fn path(&self) -> &std::path::Path {
        std::path::Path::new(self.path.as_str())
    }
}

/// 用于 RayTracing Pipeline 的创建
///
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
