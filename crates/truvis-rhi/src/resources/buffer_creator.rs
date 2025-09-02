use ash::vk;

pub struct BufferCreateInfo
{
    inner: vk::BufferCreateInfo<'static>,
    queue_family_indices: Vec<u32>,
}

impl BufferCreateInfo
{
    #[inline]
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self
    {
        Self {
            inner: vk::BufferCreateInfo {
                size,
                usage,
                ..Default::default()
            },
            queue_family_indices: Vec::new(),
        }
    }

    /// getter
    #[inline]
    pub fn info(&self) -> &vk::BufferCreateInfo<'_>
    {
        &self.inner
    }

    /// getter
    #[inline]
    pub fn size(&self) -> vk::DeviceSize
    {
        self.inner.size
    }

    /// builder
    #[inline]
    pub fn queue_family_indices(mut self, indices: &[u32]) -> Self
    {
        self.queue_family_indices = indices.to_vec();

        self.inner.queue_family_index_count = indices.len() as u32;
        self.inner.p_queue_family_indices = self.queue_family_indices.as_ptr();

        self
    }
}
