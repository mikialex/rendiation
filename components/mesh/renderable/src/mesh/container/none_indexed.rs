use crate::*;
use core::marker::PhantomData;

pub struct NoneIndexedMesh<T, U> {
  pub data: U,
  _phantom: PhantomData<T>,
}

impl<T, U> incremental::SimpleIncremental for NoneIndexedMesh<T, U>
where
  Self: Clone + Send + Sync,
{
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
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
  T::Primitive<U::Output>: PrimitiveData<U>,
{
  type Primitive = T::Primitive<U::Output>;

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.data.len() - T::STRIDE) / T::STEP + 1
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
