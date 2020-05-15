use crate::primitive::*;
use crate::vertex::Vertex;
use core::marker::PhantomData;

pub struct NoneIndexedGeometry<T: PrimitiveTopology = TriangleList> {
  data: Vec<Vertex>,
  _phantom: PhantomData<T>,
}

impl<T: PrimitiveTopology> NoneIndexedGeometry<T> {
  pub fn new<U: PrimitiveTopology>(v: Vec<Vertex>) -> Self {
    Self {
      data: v,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> PrimitiveIter<'a, T::Primitive> {
    PrimitiveIter::new(&self.data)
  }
}

impl From<Vec<Vertex>> for NoneIndexedGeometry {
  fn from(item: Vec<Vertex>) -> Self {
    Self::new::<TriangleList>(item)
  }
}
