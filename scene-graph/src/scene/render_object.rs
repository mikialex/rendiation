use crate::{GeometryHandle, Scene, SceneGraphBackend, ShadingHandle};
use arena::Handle;
use rendiation_render_entity::BoundingData;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

pub struct RenderObject<T: SceneGraphBackend> {
  pub shading_index: ShadingHandle<T>,
  pub geometry_index: GeometryHandle<T>,
  pub render_order: i32, // todo for sorting
}

impl<T: SceneGraphBackend> RenderObject<T> {
  pub fn get_bounding_local<'a>(&self, scene: &'a Scene<T>) -> &'a BoundingData {
    let geometry = scene.resources.get_geometry(self.geometry_index).resource();
    geometry.get_bounding_local()
  }
}
