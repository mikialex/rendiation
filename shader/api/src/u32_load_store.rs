use crate::*;

#[derive(Clone, Copy)]
pub struct U32BufferLoadStoreSource {
  pub array: StorageNode<[u32]>,
  pub offset: Node<u32>,
}

pub struct U32BufferLoadStore<T> {
  pub accessor: U32BufferLoadStoreSource,
  pub ty: PhantomData<T>,
}

impl<T> ShaderAbstractLeftValue for U32BufferLoadStore<T>
where
  T: ShaderSizedValueNodeType,
{
  type RightValue = Node<T>;

  fn abstract_load(&self) -> Self::RightValue {
    Node::<T>::load_from_u32_buffer(self.accessor.array, self.accessor.offset)
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    payload.store_into_u32_buffer(self.accessor.array, self.accessor.offset);
  }
}
