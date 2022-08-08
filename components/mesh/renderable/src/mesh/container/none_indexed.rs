use crate::*;
use core::marker::PhantomData;

pub struct NoneIndexedMesh<T, U> {
  pub data: U,
  _phantom: PhantomData<T>,
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
  T::Primitive<U::Output>: PrimitiveData<U>,
{
  type Primitive = T::Primitive<U::Output>;

  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.data.len()
  }

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.data.len() - T::STRIDE) / T::STEP + 1
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_data(&self.data, index)
  }
}
