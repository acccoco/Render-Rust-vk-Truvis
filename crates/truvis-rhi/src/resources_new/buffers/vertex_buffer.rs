use std::marker::PhantomData;

use crate::resources_new::resource_handles::BufferHandle;

#[derive(Debug, Clone, Copy)]
pub struct VertexBuffer<T> {
    buffer: BufferHandle,
    _phantom_data: PhantomData<T>,
}
