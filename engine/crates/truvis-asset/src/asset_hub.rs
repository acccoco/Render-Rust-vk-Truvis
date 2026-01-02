use crate::asset_loader::{AssetLoadRequest, AssetLoader, LoadResult};
use crate::asset_upload_manager::AssetUploadManager;
use crate::handle::{AssetTextureHandle, LoadStatus};
use slotmap::{SecondaryMap, SlotMap};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use truvis_gfx::resources::image::GfxImage;
use truvis_render_interface::bindless_manager::BindlessManager;
use truvis_render_interface::frame_counter::FrameCounter;
use truvis_render_interface::gfx_resource_manager::GfxResourceManager;
use truvis_render_interface::handles::GfxTextureHandle;
use truvis_render_interface::texture::GfxTexture;

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
    texture_states: SlotMap<AssetTextureHandle, LoadStatus>,

    // 存储实际的纹理资源 (仅 Ready 状态才有)
    textures: SecondaryMap<AssetTextureHandle, GfxTextureHandle>,

    // 路径到句柄的映射，用于去重 (避免重复加载同一文件)
    texture_cache: HashMap<PathBuf, AssetTextureHandle>,

    // 默认资源 (1x1 粉色纹理)，用于 Loading/Failed 状态时的占位
    fallback_texture: GfxTextureHandle,

    asset_loader: AssetLoader,
    upload_manager: AssetUploadManager,
}
// new & init
impl AssetHub {
    pub fn new(gfx_resource_manager: &mut GfxResourceManager, bindless_manager: &mut BindlessManager) -> Self {
        let fallback_texture = Self::create_fallback_texture();
        let fallback_texture_handle = gfx_resource_manager.register_texture(fallback_texture);
        bindless_manager.register_srv_with_texture(fallback_texture_handle);

        Self {
            texture_states: SlotMap::with_key(),
            textures: SecondaryMap::new(),
            texture_cache: HashMap::new(),
            fallback_texture: fallback_texture_handle,
            asset_loader: AssetLoader::new(),
            upload_manager: AssetUploadManager::new(),
        }
    }

    /// 创建一个 1x1 的粉色纹理 (同步创建)
    /// 这是一个阻塞操作，只在初始化时执行一次。
    fn create_fallback_texture() -> GfxTexture {
        // 1. Create Image (1x1 Pink)
        let pixels: [u8; 4] = [255, 0, 255, 255];
        let image = GfxImage::from_rgba8(1, 1, &pixels, "FallbackTexture");

        GfxTexture::new(image, "FallbackTexture")
    }
}
// destroy
impl AssetHub {
    pub fn destroy(self, gfx_resource_manager: &mut GfxResourceManager, frame_counter: &FrameCounter) {
        gfx_resource_manager.destroy_texture(self.fallback_texture, frame_counter.frame_id());
    }
    pub fn destroy_mut(&mut self, gfx_resource_manager: &mut GfxResourceManager, frame_counter: &FrameCounter) {
        gfx_resource_manager.destroy_texture(self.fallback_texture, frame_counter.frame_id());
    }
}
// tools
impl AssetHub {
    /// 请求加载纹理
    ///
    /// 这是一个非阻塞调用。
    /// 1. 如果已缓存，直接返回现有 Handle。
    /// 2. 如果是新请求，分配 Handle，状态设为 Loading。
    /// 3. 发送请求给后台 IO 线程。
    /// 4. 立即返回 Handle。
    pub fn load_texture(&mut self, path: PathBuf) -> AssetTextureHandle {
        let _span = tracy_client::span!("load_texture");
        if let Some(&handle) = self.texture_cache.get(&path) {
            return handle;
        }

        // 分配句柄，初始状态为 Loading
        let handle = self.texture_states.insert(LoadStatus::Loading);
        self.texture_cache.insert(path.clone(), handle);

        log::info!("Request load texture: {:?}", path);

        // 发送 IO 请求到后台线程
        self.asset_loader.request_load(AssetLoadRequest { path, handle });

        handle
    }

    pub fn get_status(&self, handle: AssetTextureHandle) -> LoadStatus {
        self.texture_states.get(handle).copied().unwrap_or(LoadStatus::Failed)
    }

    /// 获取纹理资源
    ///
    /// 如果资源已 Ready，返回实际纹理。
    /// 否则 (Loading/Uploading/Failed)，返回 Fallback 纹理。
    /// 这保证了渲染循环永远不会因为资源未就绪而阻塞或崩溃。
    pub fn get_texture(&self, asset_tex_handle: AssetTextureHandle) -> GfxTextureHandle {
        *self.textures.get(asset_tex_handle).unwrap_or(&self.fallback_texture)
    }

    pub fn get_texture_by_path(&self, tex_path: &Path) -> GfxTextureHandle {
        let asset_tex_handle = self.texture_cache.get(tex_path).unwrap();
        self.get_texture(*asset_tex_handle)
    }

    /// 驱动加载流程 (每帧调用)
    ///
    /// 1. 检查 IO 线程是否有完成的任务 -> 提交给 TransferManager。
    /// 2. 检查 TransferManager 是否有完成的上传 -> 创建 View/Sampler 并标记为 Ready。
    pub fn update(&mut self, gfx_resource_manager: &mut GfxResourceManager, bindless_manager: &mut BindlessManager) {
        let _span = tracy_client::span!("AssetHub::update");
        // 1. 处理 IO 完成的消息
        while let Some(result) = self.asset_loader.try_recv_result() {
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
                    if let Err(e) = self.upload_manager.upload_texture(data) {
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
        let finished_uploads = self.upload_manager.update();
        for (handle, image) in finished_uploads {
            log::info!("Upload finished for texture handle: {:?}", handle);

            let texture = GfxTexture::new(image, "TextureView");

            // 注册到 Bindless 里面去
            let gfx_texture_handle = gfx_resource_manager.register_texture(texture);
            bindless_manager.register_srv_with_texture(gfx_texture_handle);

            self.textures.insert(handle, gfx_texture_handle);

            if let Some(status) = self.texture_states.get_mut(handle) {
                *status = LoadStatus::Ready;
            }
        }
    }
}
