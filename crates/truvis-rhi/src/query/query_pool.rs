use std::rc::Rc;

use ash::vk;

use crate::foundation::{debug_messenger::DebugType, device::DeviceFunctions};

pub struct QueryPool
{
    handle: vk::QueryPool,
    query_type: vk::QueryType,

    /// pool 的容量
    _cnt: u32,

    device_functions: Rc<DeviceFunctions>,
}
impl DebugType for QueryPool
{
    fn debug_type_name() -> &'static str
    {
        "RhiQueryPool"
    }

    fn vk_handle(&self) -> impl vk::Handle
    {
        self.handle
    }
}
impl Drop for QueryPool
{
    fn drop(&mut self)
    {
        unsafe {
            self.device_functions.destroy_query_pool(self.handle, None);
        }
    }
}
impl QueryPool
{
    #[inline]
    pub fn new(device_functions: Rc<DeviceFunctions>, ty: vk::QueryType, cnt: u32, debug_name: &str) -> Self
    {
        let create_info = vk::QueryPoolCreateInfo {
            query_type: ty,
            query_count: cnt,
            ..Default::default()
        };

        let handle = unsafe { device_functions.create_query_pool(&create_info, None).unwrap() };

        let query_pool = Self {
            device_functions: device_functions.clone(),
            handle,
            query_type: ty,
            _cnt: cnt,
        };
        device_functions.set_debug_name(&query_pool, debug_name);
        query_pool
    }

    #[inline]
    pub fn handle(&self) -> vk::QueryPool
    {
        self.handle
    }

    #[inline]
    pub fn query_type(&self) -> vk::QueryType
    {
        self.query_type
    }

    #[inline]
    pub fn get_query_result<T: Default + Sized + Clone>(&mut self, first_index: u32, query_cnt: u32) -> Vec<T>
    {
        unsafe {
            let mut res = vec![Default::default(); query_cnt as usize];
            self.device_functions
                .get_query_pool_results(self.handle, first_index, &mut res, vk::QueryResultFlags::WAIT)
                .unwrap();
            res
        }
    }

    #[inline]
    pub fn reset(&mut self, first_query: u32, query_cnt: u32)
    {
        unsafe {
            self.device_functions.reset_query_pool(self.handle, first_query, query_cnt);
        }
    }

    #[inline]
    pub fn destroy(self)
    {
        drop(self)
    }
}
