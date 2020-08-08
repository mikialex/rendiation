use super::super::*;
use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_math_entity::Positioned3D;

pub struct NoneIndexedGeometry<
  V: Positioned3D = Vertex,
  T: PrimitiveTopology<V> = TriangleList,
  U: GeometryDataContainer<V> = Vec<V>,
> {
  pub data: U,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<V, T, U> NoneIndexedGeometry<V, T, U>
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  pub fn new(v: U) -> Self {
    Self {
      data: v,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> PrimitiveIter<'a, V, T::Primitive> {
    PrimitiveIter::new(self.data.as_ref())
  }

  pub fn get_primitive_count(&self) -> u32 {
    self.data.as_ref().len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.data.as_ref().len() as u32
  }
}
