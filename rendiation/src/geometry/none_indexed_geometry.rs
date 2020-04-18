use crate::primitive::*;
use crate::renderer::buffer::WGPUBuffer;
use crate::vertex::Vertex;
use core::marker::PhantomData;

pub struct NoneIndexedGeometry<T: PrimitiveTopology = TriangleList> {
  data: Vec<Vertex>,
  data_changed: bool,
  _phantom: PhantomData<T>,
}

impl<T: PrimitiveTopology> NoneIndexedGeometry<T> {
  pub fn new<U: PrimitiveTopology>(v: Vec<Vertex>) -> Self {
    Self {
      data: v,
      data_changed: false,
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
