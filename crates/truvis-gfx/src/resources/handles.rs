use std::marker::PhantomData;

use slotmap::new_key_type;

// 内部 Key (不直接暴露给普通用户，或者作为底层 API)
new_key_type! {
    /// 内部 Image Handle Key
    pub struct InnerImageHandle;
    /// 内部 ImageView Handle Key
    pub struct InnerImageViewHandle;
    /// 内部 Buffer Handle Key
    pub struct InnerBufferHandle;
}

// --- Buffer Handles ---

/// 通用 Buffer Handle
///
/// 指向一个 GPU Buffer 资源。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferHandle {
    pub(crate) inner: InnerBufferHandle,
}

/// 强类型顶点 Buffer Handle
///
/// 泛型 `T` 表示顶点布局类型。
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct VertexBufferHandle<T> {
    pub(crate) inner: InnerBufferHandle,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Clone for VertexBufferHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for VertexBufferHandle<T> {}

/// 索引 Buffer Handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IndexBufferHandle {
    pub(crate) inner: InnerBufferHandle,
    // 可以包含 index type 信息，或者在 resource meta 中存储
}

/// 强类型结构化 Buffer Handle (Structured Buffer / Storage Buffer)
///
/// 泛型 `T` 表示 Buffer 中存储的元素类型。
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct StructuredBufferHandle<T> {
    pub(crate) inner: InnerBufferHandle,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Clone for StructuredBufferHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StructuredBufferHandle<T> {}

// --- Image Handles ---

/// Image Handle
///
/// 指向一个 GPU Image 资源。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageHandle {
    pub(crate) inner: InnerImageHandle,
}

/// ImageView Handle
///
/// 指向一个 GPU ImageView 资源。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ImageViewHandle {
    pub(crate) inner: InnerImageViewHandle,
}
