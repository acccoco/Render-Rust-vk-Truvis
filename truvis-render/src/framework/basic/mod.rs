pub mod color;

/// frames in flight name
pub const FRAME_ID_MAP: [char; 4] = ['A', 'B', 'C', 'D'];


pub struct Camera {}

impl Camera {}

pub struct DataUtils {}

impl DataUtils
{
    /// 将任意引用类型转换为字节
    pub fn transform_u8<T: Sized>(data: &T) -> &[u8]
    {
        unsafe { std::slice::from_raw_parts(data as *const T as *const u8, size_of::<T>()) }
    }
}
