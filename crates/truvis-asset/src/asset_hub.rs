use crate::handle::{LoadStatus, TextureHandle};
use crate::loader::{AssetLoadRequest, IoWorker, LoadResult};
use crate::transfer::AssetTransferManager;
use ash::vk;
use slotmap::{SecondaryMap, SlotMap};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use truvis_gfx::gfx::Gfx;
use truvis_gfx::resources::handles::{ImageHandle, ImageViewHandle};
use truvis_gfx::sampler_manager::GfxSamplerDesc;

/// 纹理资源 (RAII)
/// 包含 Image, Allocation, ImageView, Sampler
/// Drop 时自动释放 Vulkan 资源
pub struct TextureResource {
    pub image: ImageHandle,
    pub view: ImageViewHandle,
    pub sampler: vk::Sampler,
}

impl Drop for TextureResource {
    fn drop(&mut self) {
        let mut rm = Gfx::get().resource_manager();
        rm.destroy_image_auto(self.image);
    }
}

/// 资产中心 (Facade)
///
/// 整个异步加载系统的核心协调者。
/// 职责:
/// 1. 维护所有资产的状态 (Unloaded -> Loading -> Uploading -> Ready)。
/// 2. 管理 IO 线程 (IoWorker) 和 GPU 传输 (TransferManager)。
/// 3. 提供统一的加载接口 (load_texture) 和访问接口 (get_texture)。
/// 4. 提供 Fallback 机制 (未加载完成时返回粉色纹理)。
pub struct AssetHub {
    // 存储纹理的状态
    texture_states: SlotMap<TextureHandle, LoadStatus>,

    // 存储实际的纹理资源 (仅 Ready 状态才有)
    textures: SecondaryMap<TextureHandle, Arc<TextureResource>>,

    // 路径到句柄的映射，用于去重 (避免重复加载同一文件)
    texture_cache: HashMap<PathBuf, TextureHandle>,

    // 默认资源 (1x1 粉色纹理)，用于 Loading/Failed 状态时的占位
    fallback_texture: Arc<TextureResource>,

    io_worker: IoWorker,
    transfer_manager: AssetTransferManager,
}

impl Default for AssetHub {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetHub {
    pub fn new() -> Self {
        let fallback_texture = Self::create_fallback_texture();

        Self {
            texture_states: SlotMap::with_key(),
            textures: SecondaryMap::new(),
            texture_cache: HashMap::new(),
            fallback_texture,
            io_worker: IoWorker::new(),
            transfer_manager: AssetTransferManager::new(),
        }
    }

    /// 创建一个 1x1 的粉色纹理 (同步创建)
    /// 这是一个阻塞操作，只在初始化时执行一次。
    fn create_fallback_texture() -> Arc<TextureResource> {
        // 1. Create Image (1x1 Pink)
        let pixels: [u8; 4] = [255, 0, 255, 255];

        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width: 1,
                height: 1,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let mut rm = Gfx::get().resource_manager();
        let image = rm.create_image_with_data(&create_info, &alloc_info, &pixels, "FallbackTexture");

        // Get default view
        let view = rm.get_image(image).unwrap().default_view;

        // Create Sampler
        let sampler = Gfx::get().sampler_manager().get_sampler(&GfxSamplerDesc::default());

        Arc::new(TextureResource { image, view, sampler })
    }

    /// 请求加载纹理
    ///
    /// 这是一个非阻塞调用。
    /// 1. 如果已缓存，直接返回现有 Handle。
    /// 2. 如果是新请求，分配 Handle，状态设为 Loading。
    /// 3. 发送请求给后台 IO 线程。
    /// 4. 立即返回 Handle。
    pub fn load_texture(&mut self, path: PathBuf) -> TextureHandle {
        let _span = tracy_client::span!("load_texture");
        if let Some(&handle) = self.texture_cache.get(&path) {
            return handle;
        }

        // 分配句柄，初始状态为 Loading
        let handle = self.texture_states.insert(LoadStatus::Loading);
        self.texture_cache.insert(path.clone(), handle);

        log::info!("Request load texture: {:?}", path);

        // 发送 IO 请求到后台线程
        self.io_worker.request_load(AssetLoadRequest { path, handle });

        handle
    }

    pub fn get_status(&self, handle: TextureHandle) -> LoadStatus {
        self.texture_states.get(handle).copied().unwrap_or(LoadStatus::Failed)
    }

    /// 获取纹理资源
    ///
    /// 如果资源已 Ready，返回实际纹理。
    /// 否则 (Loading/Uploading/Failed)，返回 Fallback 纹理。
    /// 这保证了渲染循环永远不会因为资源未就绪而阻塞或崩溃。
    pub fn get_texture(&self, handle: TextureHandle) -> Arc<TextureResource> {
        self.textures.get(handle).cloned().unwrap_or_else(|| self.fallback_texture.clone())
    }

    pub fn iter_handles(&self) -> impl Iterator<Item = TextureHandle> + '_ {
        self.texture_states.keys()
    }

    /// 驱动加载流程 (每帧调用)
    ///
    /// 1. 检查 IO 线程是否有完成的任务 -> 提交给 TransferManager。
    /// 2. 检查 TransferManager 是否有完成的上传 -> 创建 View/Sampler 并标记为 Ready。
    pub fn update(&mut self) {
        let _span = tracy_client::span!("AssetHub::update");
        // 1. 处理 IO 完成的消息
        while let Some(result) = self.io_worker.try_recv_result() {
            match result {
                LoadResult::Success(data) => {
                    let handle = data.handle;
                    log::info!(
                        "IO finished for texture handle: {:?}, size: {}x{}",
                        handle,
                        data.extent.width,
                        data.extent.height
                    );

                    if let Some(status) = self.texture_states.get_mut(handle) {
                        *status = LoadStatus::Uploading;
                    }

                    // 提交给 TransferManager (CPU -> GPU)
                    if let Err(e) = self.transfer_manager.upload_texture(data) {
                        log::error!("Failed to submit upload task: {}", e);
                        if let Some(status) = self.texture_states.get_mut(handle) {
                            *status = LoadStatus::Failed;
                        }
                    }
                }
                LoadResult::Failure(handle, err) => {
                    log::error!("IO failed for texture handle: {:?}, error: {}", handle, err);
                    if let Some(status) = self.texture_states.get_mut(handle) {
                        *status = LoadStatus::Failed;
                    }
                }
            }
        }

        // 2. 检查 GPU 上传完成
        let finished_uploads = self.transfer_manager.update();
        for (handle, image) in finished_uploads {
            log::info!("Upload finished for texture handle: {:?}", handle);

            // ImageHandle already has a default view created by ResourceManager
            let rm = Gfx::get().resource_manager();
            let view = rm.get_image(image).unwrap().default_view;

            // Create Sampler (TODO: Use params from load request)
            let sampler = Gfx::get().sampler_manager().get_sampler(&GfxSamplerDesc::default());

            let resource = Arc::new(TextureResource { image, view, sampler });

            self.textures.insert(handle, resource);

            if let Some(status) = self.texture_states.get_mut(handle) {
                *status = LoadStatus::Ready;
            }
        }
    }
}
