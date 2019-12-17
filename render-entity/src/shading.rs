use crate::geometry::Geometry;
use std::rc::Rc;
use std::hash::Hash;
use std::hash::Hasher;

pub trait Shading<Renderer> {
  fn get_index(&self) -> usize;
  fn get_vertex_str(&self) -> &str;
  fn get_fragment_str(&self) -> &str;
  fn make_gpu_port(&self, renderer: &Renderer) -> Rc<dyn ShadingGPUPort<Renderer>>;
}

pub trait ShadingGPUPort<Renderer> {
  fn get_index(&self) -> usize;
  fn use_self(&self, renderer: &Renderer);
  fn use_uniforms(&self, renderer: &Renderer);
  fn use_geometry(&self, renderer: &mut Renderer, geometry: Rc<dyn Geometry>);
}

impl<Renderer> Hash for dyn Shading<Renderer> {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.get_index().hash(state);
  }
}

impl<Renderer> PartialEq for dyn Shading<Renderer> {
  fn eq(&self, other: &Self) -> bool {
    self.get_index() == other.get_index()
  }
}
impl<Renderer> Eq for dyn Shading<Renderer> {}


impl<Renderer> PartialEq for dyn ShadingGPUPort<Renderer> {
  fn eq(&self, other: &Self) -> bool {
    self.get_index() == other.get_index()
  }
}
impl<Renderer> Eq for dyn ShadingGPUPort<Renderer> {}
