use crate::vertex::Vertex;

pub mod plane;
pub mod sphere;

pub trait IndexedBufferTessellator<T = Vertex> {
  type TessellationParameter;
  fn create_mesh(&self, p: &Self::TessellationParameter) -> (Vec<T>, Vec<u16>);
}

pub trait BufferTessellator<T = Vertex> {
  type TessellationParameter;
  fn create_mesh(&self, p: &Self::TessellationParameter) -> Vec<T>;
}

pub trait IndexedGeometryBuilder<T = Vertex>{
  fn add_vertex(&mut self, v: T) -> &mut Self;
}