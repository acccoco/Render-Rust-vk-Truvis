use std::ffi::CString;

use ash::vk;
use vk_mem::Alloc;

use crate::{rhi::Rhi, rhi_type::command_pool::RhiCommandPool};


// 工具方法
impl Rhi
{
    pub(crate) fn set_debug_name<T, S>(&self, handle: T, name: S)
    where
        T: vk::Handle + Copy,
        S: AsRef<str>,
    {
        let name = if name.as_ref().is_empty() { "nameless" } else { name.as_ref() };
        let name = CString::new(name).unwrap();
        unsafe {
            self.debug_util_pf
                .as_ref()
                .unwrap()
                .set_debug_utils_object_name(
                    self.device.as_ref().unwrap().handle(),
                    &vk::DebugUtilsObjectNameInfoEXT::builder()
                        .object_name(name.as_c_str())
                        .object_type(T::TYPE)
                        .object_handle(handle.as_raw()),
                )
                .unwrap();
        }
    }

    pub fn set_debug_label(&self)
    {
        todo!()
        // self.debug_util_pf.unwrap().cmd_begin_debug_utils_label()
    }

    pub fn create_command_pool<S: AsRef<str> + Clone>(
        &self,
        queue_flags: vk::QueueFlags,
        flags: vk::CommandPoolCreateFlags,
        debug_name: S,
    ) -> Option<RhiCommandPool>
    {
        let queue_family_index = self.physical_device().find_queue_family_index(queue_flags)?;

        let pool = unsafe {
            self.device()
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index).flags(flags),
                    None,
                )
                .unwrap()
        };

        self.set_debug_name(pool, debug_name);
        Some(RhiCommandPool {
            command_pool: pool,
            queue_family_index,
        })
    }

    pub fn create_image<S>(&self, create_info: &vk::ImageCreateInfo, debug_name: S) -> (vk::Image, vk_mem::Allocation)
    where
        S: AsRef<str>,
    {
        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };
        let (image, allocation) = unsafe { self.vma().create_image(create_info, &alloc_info).unwrap() };

        self.set_debug_name(image, debug_name);
        (image, allocation)
    }

    #[inline]
    pub fn create_image_view<S>(&self, create_info: &vk::ImageViewCreateInfo, debug_name: S) -> vk::ImageView
    where
        S: AsRef<str>,
    {
        let view = unsafe { self.device().create_image_view(create_info, None).unwrap() };

        self.set_debug_name(view, debug_name);
        view
    }


    pub(crate) fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Vec<vk::Format>
    {
        candidates
            .iter()
            .filter(|f| {
                let props = unsafe {
                    self.vk_instance().get_physical_device_format_properties(self.physical_device().vk_pdevice, **f)
                };
                match tiling {
                    vk::ImageTiling::LINEAR => props.linear_tiling_features.contains(features),
                    vk::ImageTiling::OPTIMAL => props.optimal_tiling_features.contains(features),
                    _ => panic!("not supported tiling."),
                }
            })
            .copied()
            .collect()
    }
}
