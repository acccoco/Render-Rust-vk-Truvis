use std::rc::Rc;

use ash::vk;

use crate::core::debug_utils::RhiDebugType;
use crate::{core::device::RhiDevice, rhi::Rhi};

pub struct RhiQueryPool {
    handle: vk::QueryPool,
    query_type: vk::QueryType,

    /// pool 的容量
    _cnt: u32,

    device: Rc<RhiDevice>,
}
impl RhiDebugType for RhiQueryPool {
    fn debug_type_name() -> &'static str {
        "RhiQueryPool"
    }

    fn vk_handle(&self) -> impl vk::Handle {
        self.handle
    }
}
impl Drop for RhiQueryPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_query_pool(self.handle, None);
        }
    }
}
impl RhiQueryPool {
    #[inline]
    pub fn new(rhi: &Rhi, ty: vk::QueryType, cnt: u32, debug_name: &str) -> Self {
        let create_info = vk::QueryPoolCreateInfo {
            query_type: ty,
            query_count: cnt,
            ..Default::default()
        };

        let handle = unsafe { rhi.device.create_query_pool(&create_info, None).unwrap() };

        let query_pool = Self {
            device: rhi.device.clone(),
            handle,
            query_type: ty,
            _cnt: cnt,
        };
        rhi.device.debug_utils().set_debug_name(&query_pool, debug_name);
        query_pool
    }

    #[inline]
    pub fn handle(&self) -> vk::QueryPool {
        self.handle
    }

    #[inline]
    pub fn query_type(&self) -> vk::QueryType {
        self.query_type
    }

    #[inline]
    pub fn get_query_result<T: Default + Sized + Clone>(&mut self, first_index: u32, query_cnt: u32) -> Vec<T> {
        unsafe {
            let mut res = vec![Default::default(); query_cnt as usize];
            self.device.get_query_pool_results(self.handle, first_index, &mut res, vk::QueryResultFlags::WAIT).unwrap();
            res
        }
    }

    #[inline]
    pub fn reset(&mut self, first_query: u32, query_cnt: u32) {
        unsafe {
            self.device.reset_query_pool(self.handle, first_query, query_cnt);
        }
    }

    #[inline]
    pub fn destroy(self) {
        drop(self)
    }
}
