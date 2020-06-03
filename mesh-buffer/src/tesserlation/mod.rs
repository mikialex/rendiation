use crate::vertex::Vertex;

pub mod plane;
pub mod sphere;

pub trait IndexedBufferTesserlator<T = Vertex> {
  type TesserlationParameter;
  fn create_mesh(&self, p: &Self::TesserlationParameter) -> (Vec<T>, Vec<u16>);
}

pub trait BufferTesserlator<T = Vertex> {
  type TesserlationParameter;
  fn create_mesh(&self, p: &Self::TesserlationParameter) -> Vec<T>;
}

pub trait IndexedGeometryBuilder<T = Vertex>{
  fn add_vertex(&mut self, v: T) -> &mut Self;
}