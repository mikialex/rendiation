use super::super::*;
use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_math_entity::PositionedPoint3;

pub struct NoneIndexedGeometry<V: PositionedPoint3 = Vertex, T: PrimitiveTopology<V> = TriangleList>
{
  pub data: Vec<V>,
  _phantom: PhantomData<T>,
}

impl<V: PositionedPoint3, T: PrimitiveTopology<V>> NoneIndexedGeometry<V, T> {
  pub fn new(v: Vec<V>) -> Self {
    Self {
      data: v,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> PrimitiveIter<'a, V, T::Primitive> {
    PrimitiveIter::new(&self.data)
  }

  pub fn get_primitive_count(&self) -> u32 {
    self.data.len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.data.len() as u32
  }
}
