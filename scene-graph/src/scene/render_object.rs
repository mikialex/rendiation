use crate::{GeometryHandle, Scene, SceneGraphBackend, ShadingHandle};
use rendiation_render_entity::BoundingData;
use arena::Handle;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

pub struct RenderObject<T: SceneGraphBackend> {
  pub shading_index: ShadingHandle<T>,
  pub geometry_index: GeometryHandle<T>,
  pub render_order: i32, // todo for sorting
}

impl<T: SceneGraphBackend> RenderObject<T> {
  pub fn get_bounding_local(&self, _scene: &Scene<T>) -> &BoundingData {
    todo!()
  }
}
