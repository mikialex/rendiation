use crate::vertex::Vertex;

pub mod plane;
pub mod sphere;

pub trait IndexedBufferMesher {
  fn create_mesh(&self) -> (Vec<Vertex>, Vec<u16>);
}
