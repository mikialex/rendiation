use crate::*;

pub struct NoneIndexedMesh<T, U> {
  pub data: U,
  _phantom: PhantomData<T>,
}

impl<T, U: Clone> Clone for NoneIndexedMesh<T, U> {
  fn clone(&self) -> Self {
    Self {
      data: self.data.clone(),
      _phantom: self._phantom,
    }
  }
}

impl<T, U> NoneIndexedMesh<T, U> {
  pub fn new(v: U) -> Self {
    Self {
      data: v,
      _phantom: PhantomData,
    }
  }
}

impl<T, U> AbstractMesh for NoneIndexedMesh<T, U>
where
  T: PrimitiveTopologyMeta,
  U: VertexContainer,
  U::Output: Copy,
  T::Primitive<U::Output>: PrimitiveData<U>,
{
  type Primitive = T::Primitive<U::Output>;

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.data.len() + T::STEP - T::STRIDE) / T::STEP
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let index = primitive_index * T::STEP;
    T::Primitive::from_data(&self.data, index)
  }

  #[inline(always)]
  unsafe fn primitive_at_unchecked(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_data_unchecked(&self.data, index)
  }
}

impl<T, U: CollectionSize> GPUConsumableMeshBuffer for NoneIndexedMesh<T, U> {
  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.data.len()
  }
}
