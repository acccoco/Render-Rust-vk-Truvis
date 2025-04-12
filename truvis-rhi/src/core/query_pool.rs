use std::rc::Rc;

use ash::vk;

use crate::{core::device::RhiDevice, rhi::Rhi};

pub struct QueryPool {
    pub(crate) handle: vk::QueryPool,
    pub(crate) query_type: vk::QueryType,

    /// pool 的容量
    _cnt: u32,

    device: Rc<RhiDevice>,
}

impl QueryPool {
    #[inline]
    pub fn new(rhi: &Rhi, ty: vk::QueryType, cnt: u32, debug_name: &str) -> Self {
        let create_info = vk::QueryPoolCreateInfo {
            query_type: ty,
            query_count: cnt,
            ..Default::default()
        };

        unsafe {
            let handle = rhi.device.create_query_pool(&create_info, None).unwrap();
            rhi.device.debug_utils.set_object_debug_name(handle, debug_name);

            Self {
                device: rhi.device.clone(),
                handle,
                query_type: ty,
                _cnt: cnt,
            }
        }
    }

    // #[inline]
    pub fn get_query_result<T>(&mut self, first_index: u32, query_cnt: u32) -> Vec<T>
    where
        T: Default + Sized + Clone,
    {
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
        unsafe {
            self.device.destroy_query_pool(self.handle, None);
        }
    }
}
