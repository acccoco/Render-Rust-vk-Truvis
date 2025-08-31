use crate::resources::resource_handles::RhiBufferHandle;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct RhiVertexBuffer<T> {
    buffer: RhiBufferHandle,
    _phantom_data: PhantomData<T>,
}
