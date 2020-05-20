use crate::{Index, ResourceManager, SceneGraphBackEnd};
use std::ops::Range;

pub trait Geometry<T: SceneGraphBackEnd> {
  fn update_gpu(&mut self, renderer: &mut T::Renderer);
  fn get_gpu_index_buffer(&self) -> & T::IndexBuffer;
  fn get_gpu_vertex_buffer(&self, index: usize) -> & T::VertexBuffer;
  fn vertex_buffer_count(&self) -> usize;
  fn get_draw_range(&self) -> Range<u32>;
}

pub struct SceneGeometry<T: SceneGraphBackEnd> {
  index: Index,
  pub data: Box<dyn Geometry<T>>,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn create_geometry(&mut self, geometry: impl Geometry<T> + 'static) -> &mut SceneGeometry<T> {
    let wrapped = SceneGeometry {
      index: Index::from_raw_parts(0, 0),
      data: Box::new(geometry),
    };
    let index = self.geometries.insert(wrapped);
    let g = self.get_geometry_mut(index);
    g.index = index;
    g
  }

  pub fn get_geometry_mut(&mut self, index: Index) -> &mut SceneGeometry<T> {
    self.geometries.get_mut(index).unwrap()
  }

  pub fn get_geometry(&self, index: Index) -> &SceneGeometry<T> {
    self.geometries.get(index).unwrap()
  }

  pub fn delete_geometry(&mut self, index: Index) {
    self.geometries.remove(index);
  }
}
