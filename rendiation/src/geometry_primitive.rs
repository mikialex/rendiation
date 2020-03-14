use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_math_entity::Face;

pub trait PrimitiveFromGeometryData {
  fn from_data(index: &[u16], data: &[Vertex], offset: usize) -> Self;
}

impl PrimitiveFromGeometryData for Face {
  fn from_data(index: &[u16], data: &[Vertex], offset: usize) -> Self {
    let a = data[index[offset] as usize].position;
    let b = data[index[offset + 1] as usize].position;
    let c = data[index[offset + 2] as usize].position;
    Face { a, b, c }
  }
}

pub trait PrimitiveTopology {
  type Primitive: PrimitiveFromGeometryData;
  const STRIDE: usize;
}

pub struct TriangleList;

impl PrimitiveTopology for TriangleList {
  type Primitive = Face;
  const STRIDE: usize = 3;
}

pub struct PrimitiveIter<'a, T: PrimitiveFromGeometryData> {
  pub index: &'a [u16],
  pub data: &'a [Vertex],
  pub current: usize,
  pub _phantom: PhantomData<T>,
}


impl<'a, T: PrimitiveFromGeometryData> Iterator for PrimitiveIter<'a, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    if self.current == self.index.len() {
      None
    } else {
      Some(T::from_data(self.index, self.data, self.current))
    }
  }
}
