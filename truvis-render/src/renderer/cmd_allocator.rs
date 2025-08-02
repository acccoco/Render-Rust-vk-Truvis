use std::rc::Rc;
use ash::vk;
use itertools::Itertools;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::command_pool::RhiCommandPool;
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::rhi::Rhi;
use crate::pipeline_settings::FrameSettings;
use crate::renderer::frame_controller::FrameController;

pub struct CmdAllocator {
    /// 为每个 frame 分配一个 command pool
    graphics_command_pools: Vec<Rc<RhiCommandPool>>,

    /// 每个 command pool 已经分配出去的 command buffer，用于集中 free 或其他操作
    allocated_command_buffers: Vec<Vec<RhiCommandBuffer>>,

    frame_ctrl: Rc<FrameController>,

    device: Rc<RhiDevice>,
}

impl CmdAllocator {
    pub fn new(rhi: &Rhi, frame_settings: &FrameSettings, frame_ctrl: Rc<FrameController>) -> Self {
        let graphics_command_pools = (0..frame_settings.fif_num)
            .map(|i| {
                Rc::new(RhiCommandPool::new(
                    rhi.device.clone(),
                    rhi.graphics_queue_family(),
                    vk::CommandPoolCreateFlags::TRANSIENT,
                    &format!("render_context_graphics_command_pool_{}", i),
                ))
            })
            .collect_vec();

        Self {
            graphics_command_pools,
            allocated_command_buffers: vec![Vec::new(); frame_settings.fif_num],
            device: rhi.device.clone(),
            frame_ctrl,
        }
    }

    /// 分配 command buffer，在当前 frame 使用
    pub fn alloc_command_buffer(&mut self, debug_name: &str) -> RhiCommandBuffer {
        let name = format!("[{}]{}", self.frame_ctrl.frame_name(), debug_name);
        let cmd = RhiCommandBuffer::new(
            self.device.clone(),
            self.graphics_command_pools[*self.frame_ctrl.frame_label()].clone(),
            &name,
        );

        self.allocated_command_buffers[*self.frame_ctrl.frame_label()].push(cmd.clone());
        cmd
    }

    pub fn free_frame_commands(&mut self) {
        // 释放当前 frame 的 command buffer 的资源
        std::mem::take(&mut self.allocated_command_buffers[*self.frame_ctrl.frame_label()]) //
            .into_iter()
            .for_each(|cmd| cmd.free());

        // 这个调用并不会释放资源，而是将 pool 内的 command buffer 设置到初始状态
        self.graphics_command_pools[*self.frame_ctrl.frame_label()].reset_all_buffers();
    }
}