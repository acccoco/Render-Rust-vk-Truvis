use ash::vk;

use crate::rhi::Rhi;

pub struct RhiQueryPool
{
    pub(crate) handle: vk::QueryPool,
    pub(crate) query_type: vk::QueryType,

    /// pool 的容量
    pub(crate) cnt: u32,
}

impl RhiQueryPool
{
    #[inline]
    pub fn new(ty: vk::QueryType, cnt: u32, debug_name: &str) -> Self
    {
        let create_info = vk::QueryPoolCreateInfo {
            query_type: ty,
            query_count: cnt,
            ..Default::default()
        };

        let rhi = Rhi::instance();

        unsafe {
            let handle = rhi.device().create_query_pool(&create_info, None).unwrap();
            rhi.set_debug_name(handle, debug_name);

            Self {
                handle,
                query_type: ty,
                cnt,
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
            Rhi::instance()
                .device()
                .get_query_pool_results(self.handle, first_index, query_cnt, &mut res, vk::QueryResultFlags::WAIT)
                .unwrap();
            res
        }
    }

    #[inline]
    pub fn reset(&mut self, first_query: u32, query_cnt: u32)
    {
        unsafe {
            Rhi::instance().device().reset_query_pool(self.handle, first_query, query_cnt);
        }
    }

    #[inline]
    pub fn destroy(self)
    {
        unsafe {
            Rhi::instance().device().destroy_query_pool(self.handle, None);
        }
    }
}