/// > from Vulkan Samples
/// >
/// > RenderFrame is a container for per-frame data, including BufferPool objects,
/// > synchronization primitives (semaphores, fences) and the swapchain RenderTarget.
/// >
/// > When creating a RenderTarget, we need to provide images that will be used as attachments
/// > within a RenderPass. The RenderFrame is responsible for creating a RenderTarget using
/// > RenderTarget::CreateFunc. A custom RenderTarget::CreateFunc can be provided if a different
/// > render target is required.
/// >
/// > A RenderFrame cannot be destroyed individually since frames are managed by the RenderContext,
/// > the whole context must be destroyed. This is because each RenderFrame holds Vulkan objects
/// > such as the swapchain image.
pub struct RenderFrmae {}
