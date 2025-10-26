use crate::renderer::bindless::BindlessManager;
use crate::renderer::cmd_allocator::CmdAllocator;
use crate::renderer::frame_controller::FrameController;
use crate::renderer::stage_buffer_manager::StageBufferManager;
use ash::vk;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use truvis_rhi::commands::command_buffer::CommandBuffer;
use truvis_rhi::commands::command_pool::CommandPool;
use truvis_rhi::commands::submit_info::SubmitInfo;
use truvis_rhi::render_context::RenderContext;

pub struct FrameContext {
    pub frame_ctrl: Rc<FrameController>,
    pub upload_buffer_mgr: RefCell<StageBufferManager>,
    pub bindless_mgr: RefCell<BindlessManager>,
    pub cmd_allocator: RefCell<CmdAllocator>,
}

static mut FRAME_CONTEXT: Option<FrameContext> = None;

// init & destroy
impl FrameContext {
    fn new() -> Self {
        let frame_ctrl = Rc::new(FrameController::new());
        let upload_buffer_mgr = RefCell::new(StageBufferManager::new(frame_ctrl.clone()));
        let bindless_mgr = RefCell::new(BindlessManager::new(frame_ctrl.clone()));
        let cmd_allocator = RefCell::new(CmdAllocator::new(frame_ctrl.clone()));
        Self {
            frame_ctrl,
            upload_buffer_mgr,
            bindless_mgr,
            cmd_allocator,
        }
    }

    pub fn get() -> &'static FrameContext {
        unsafe {
            // 使用 addr_of! 避免直接对 static mut 创建引用，编译器不允许这种行为
            let ptr = std::ptr::addr_of!(FRAME_CONTEXT);
            (*ptr).as_ref().expect("FrameContext not initialized. Call FrameContext::init() first.")
        }
    }

    pub fn init() {
        unsafe {
            // 使用 addr_of! 避免直接对 static mut 创建引用，编译器不允许这种行为
            let ptr = std::ptr::addr_of_mut!(FRAME_CONTEXT);
            assert!((*ptr).is_none(), "FrameContext already initialized");
            *ptr = Some(Self::new());
        }
    }

    pub fn destroy() {
        unsafe {
            // 使用 addr_of_mut! 避免直接对 static mut 创建可变引用
            let ptr = std::ptr::addr_of_mut!(FRAME_CONTEXT);
            let mut context = (*ptr).take().expect("FrameContext not initialized");
            drop(context.upload_buffer_mgr);
            drop(context.cmd_allocator);
            drop(context.bindless_mgr);
        }
    }
}

// getter
impl FrameContext {
    #[inline]
    pub fn bindless_mgr_mut() -> RefMut<'static, BindlessManager> {
        let context = Self::get();
        context.bindless_mgr.borrow_mut()
    }

    #[inline]
    pub fn bindless_mgr() -> Ref<'static, BindlessManager> {
        let context = Self::get();
        context.bindless_mgr.borrow()
    }

    #[inline]
    pub fn cmd_allocator_mut() -> RefMut<'static, CmdAllocator> {
        let context = Self::get();
        context.cmd_allocator.borrow_mut()
    }

    #[inline]
    pub fn stage_buffer_manager() -> RefMut<'static, StageBufferManager> {
        let context = Self::get();
        context.upload_buffer_mgr.borrow_mut()
    }
}
