#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RhiImageHandle(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RhiBufferHandle(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RhiImageViewHandle(pub(crate) u64);
