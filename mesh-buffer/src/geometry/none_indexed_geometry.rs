use super::*;
use crate::vertex::Vertex;
use core::marker::PhantomData;

pub struct NoneIndexedGeometry<V: PositionedPoint = Vertex, T: PrimitiveTopology<V> = TriangleList>
{
  data: Vec<V>,
  _phantom: PhantomData<T>,
}

impl<V: PositionedPoint, T: PrimitiveTopology<V>> NoneIndexedGeometry<V, T> {
  pub fn new(v: Vec<V>) -> Self {
    Self {
      data: v,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> PrimitiveIter<'a, V, T::Primitive> {
    PrimitiveIter::new(&self.data)
  }
}
