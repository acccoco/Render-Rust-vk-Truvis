use ash::vk;
use truvis_gfx::{gfx::Gfx, resources::texture::GfxTexture2D};

pub mod fif_buffer;

pub struct ImageLoader {}

impl ImageLoader {
    pub fn load_image(tex_path: &std::path::Path) -> GfxTexture2D {
        let img = image::ImageReader::open(tex_path).unwrap().decode().unwrap().to_rgba8();
        let width = img.width();
        let height = img.height();
        let data = img.as_raw();
        let name = tex_path.to_str().unwrap();

        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let alloc_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice,
            ..Default::default()
        };

        let mut resource_manager = Gfx::get().resource_manager();
        let image_handle = resource_manager.create_image_with_data(&create_info, &alloc_info, data, name);

        let image_res = resource_manager.get_image(image_handle).unwrap();
        let view_handle = image_res.default_view;

        GfxTexture2D::new(image_handle, view_handle, name)
    }
}
