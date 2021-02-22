use super::super::*;
use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_geometry::Positioned;

pub struct NoneIndexedGeometry<V = Vertex, T = TriangleList, U = Vec<V>> {
  pub data: U,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<V, T, U> NoneIndexedGeometry<V, T, U> {
  pub fn new(v: U) -> Self {
    Self {
      data: v,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }
}

impl<V, T, U> AnyGeometry for NoneIndexedGeometry<V, T, U>
where
  V: Positioned<f32, 3>,
  T: PrimitiveTopologyMeta<V>,
  U: GeometryDataContainer<V>,
  T::Primitive: PrimitiveData<V, U>,
{
  type Primitive = T::Primitive;

  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.data.as_ref().len()
  }

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.data.as_ref().len() - T::STRIDE) / T::STEP + 1
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_data(&self.data, index)
  }
}
