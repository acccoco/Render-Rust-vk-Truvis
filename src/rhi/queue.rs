use bitflags::bitflags;

/// 某个 queue family 对 present 的支持情况
#[derive(PartialEq)]
pub enum RhiQueueFamilyPresentProps
{
    Supported,
    NoSupported,

    /// surface 不存在，无法判断是否支持 surface
    NoSurface,
}

/// 某个 queue family 的能力
pub struct RhiQueueFamilyProps
{
    pub compute: bool,
    pub graphics: bool,
    pub present: RhiQueueFamilyPresentProps,
    pub transfer: bool,
}


bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RhiQueueType : u32{
        const Graphics = 1;
        const Compute = 2;
        const Present = 3;
        const Transfer = 4;
    }
}
