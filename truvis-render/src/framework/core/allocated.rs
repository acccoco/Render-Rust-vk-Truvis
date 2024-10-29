use ash::vk;
use vk_mem::Alloc;

static mut VMA: Option<vk_mem::Allocator> = None;

pub struct AllocatedBase
{
    alloc_create_info: vk_mem::AllocationCreateInfo,
    allocation: vk_mem::Allocation,
    mapped_data: Option<*mut u8>,

    /// vk::memory_property_host_coherent
    coherent: bool,

    /// 是否是 persistently mapped
    persistent: bool,
}

impl AllocatedBase
{
    pub fn get_memory_allocator() -> &'static vk_mem::Allocator
    {
        assert!(unsafe { VMA.is_some() }, "VMA not initialized");
        unsafe { VMA.as_ref().unwrap() }
    }

    /// 分配内存，创建 Image，返回创建的 Image
    pub fn create_image(&mut self, create_info: &vk::ImageCreateInfo) -> vk::Image
    {
        assert!(0 < create_info.mip_levels, "Images should have at least one level");
        assert!(0 < create_info.array_layers, "Images should have at least one layer");

        let allocator = Self::get_memory_allocator();
        unsafe {
            let (image, allocation) = allocator
                .create_image(create_info, &self.alloc_create_info)
                .expect("cannot create image");
            let allocation_info = allocator.get_allocation_info(&allocation).unwrap();

            self.post_create(&allocation_info);

            image
        }
    }

    fn post_create(&mut self, allocation_info: &vk_mem::AllocationInfo)
    {
        let allocator = Self::get_memory_allocator();
        unsafe {
            // let gpu_memory_properties = allocator.map_memory();

            // self.coherent = gpu_memory_properties.
            todo!()
        }
    }
}

pub trait IAllcated {}
