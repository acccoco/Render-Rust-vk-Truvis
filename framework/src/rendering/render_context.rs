use std::{cell::RefCell, rc::Rc};

use ash::vk;

use crate::{
    core::{
        device::Device,
        queue::Queue,
        swapchain::{Swapchain, SwapchainProperties},
    },
    platform::window::Window,
    rendering::render_frame::RenderFrmae,
};

/// > from Vulkan Sample
/// >
/// > RenderContext acts as a frame manager for the sample, with a lifetime that is the
/// >  same as that of the Application itself. It acts as a container for RenderFrame objects,
/// >  swapping between them (begin_frame, end_frame) and forwarding requests for Vulkan resources
/// >  to the active frame. Note that it's guaranteed that there is always an active frame.
/// >  More than one frame can be in-flight in the GPU, thus the need for per-frame resources.
/// >
/// >  It requires a Device to be valid on creation, and will take control of a given Swapchain.
/// >
/// >  For normal rendering (using a swapchain), the RenderContext can be created by passing in a
/// >  swapchain. A RenderFrame will then be created for each Swapchain image.
pub struct RenderContext
{
    surface_extent: vk::Extent2D,

    device: Rc<RefCell<Device>>,

    window: Rc<dyn Window>,

    queue: Rc<Queue>,

    swapchain: Swapchain,

    swapchain_properties: SwapchainProperties,

    frames: Vec<RenderFrmae>,

    acquired_semaphore: vk::Semaphore,

    prepared: bool,
    
    /// Current active frame index
    active_frame_index: u32,
    
    /// Whether a frame is active or not
    frame_active: bool,
}
