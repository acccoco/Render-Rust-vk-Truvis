use ash::vk;
use std::ffi::CStr;
use std::rc::Rc;

use crate::{
    commands::command_queue::CommandQueue,
    foundation::{
        debug_messenger::DebugMsger, device::GfxDevice, instance::GfxInstance, physical_device::GfxPhysicalDevice,
    },
};

pub struct GfxCore {
    /// vk 基础函数的接口
    ///
    /// 在 drop 之后，会卸载 dll，因此需要确保该字段最后 drop
    pub(crate) vk_entry: ash::Entry,

    pub(crate) instance: GfxInstance,
    pub(crate) physical_device: GfxPhysicalDevice,

    /// Vulkan 设备函数指针集合
    ///
    /// 使用 Rc 是合理的，因为：
    /// 1. 多个组件需要共享相同的设备函数指针（GfxQueue、GfxCommandBuffer 等）
    /// 2. 函数指针本身很轻量，共享比传递更高效
    /// 3. 设备生命周期需要精确控制，Rc 确保在所有引用者销毁前设备不被销毁
    ///
    /// 使用 Rc<> 的时机：在 RenderContext 内部的对象，可以通过 Rc 去访问 DevcesFunctions
    pub(crate) device_functions: Rc<GfxDevice>,

    pub(crate) debug_utils: DebugMsger,

    pub(crate) gfx_queue: CommandQueue,
}

// 创建与销毁
impl GfxCore {
    pub fn new(app_name: String, engine_name: String, instance_extra_exts: Vec<&'static CStr>) -> Self {
        let vk_pf = unsafe { ash::Entry::load() }.expect("Failed to load vulkan entry");
        let instance = GfxInstance::new(&vk_pf, app_name, engine_name, instance_extra_exts);
        let physical_device = GfxPhysicalDevice::new_descrete_physical_device(instance.ash_instance());

        // Nvidia 使用的是 Unified Scheduler，因此 Graphics 和 Compute 并没法做到真正的并行
        // Graphics 和 Compute 会争夺 SM，L2 以及显存
        // 驱动层给出了专用的 compute queue family，但是底层硬件资源依然是共享的
        // Transfer(DMA) 可以做到部分并行，不过为了简化设计，仍然然使用同一个 queue family
        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(physical_device.gfx_queue_family.queue_family_index)
            .queue_priorities(&[1.0])];

        let device = Rc::new(GfxDevice::new(&instance.ash_instance, physical_device.vk_handle, &queue_create_infos));
        let gfx_queue = CommandQueue {
            vk_queue: unsafe { device.get_device_queue(physical_device.gfx_queue_family.queue_family_index, 0) },
            queue_family: physical_device.gfx_queue_family.clone(),
            device_functions: device.clone(),
        };

        let debug_utils = DebugMsger::new(&vk_pf, &instance.ash_instance);

        log::info!("gfx queue's queue family:\n{:#?}", gfx_queue.queue_family);

        // 在 device 以及 debug_utils 之前创建的 vk::Handle
        {
            device.set_object_debug_name(instance.vk_instance(), "GfxInstance");
            device.set_object_debug_name(physical_device.vk_handle, "GfxPhysicalDevice");

            device.set_object_debug_name(device.vk_handle(), "GfxDevice");
            device.set_object_debug_name(gfx_queue.vk_queue, "CommandQueue-gfx");
        }

        Self {
            vk_entry: vk_pf,
            instance,
            physical_device,
            device_functions: device,
            debug_utils,
            gfx_queue,
        }
    }

    pub fn destroy(self) {
        self.debug_utils.destroy();
        self.device_functions.destroy();
        self.physical_device.destroy();
        self.instance.destroy();
    }
}
