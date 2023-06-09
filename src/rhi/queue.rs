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
}


pub enum RhiQueueType
{
    Graphics,
    Compute,
    Present,
}
