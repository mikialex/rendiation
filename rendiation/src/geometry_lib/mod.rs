use crate::vertex::Vertex;

pub mod sphere_geometry;
pub mod plane_geometry;

pub trait IndexedBufferMesher {
  fn create_mesh(&self) -> (Vec<Vertex>, Vec<u16>);
}
