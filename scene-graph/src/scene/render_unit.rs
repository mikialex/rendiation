use crate::{
  GeometryHandle, RALBackend, RenderObjectHandle, Scene, SceneBackend, SceneNodeHandle,
  ShadingHandle,
};
use rendiation_ral::{RenderObject, ShadingProvider};

impl<T: RALBackend, S: SceneBackend<T>> Scene<T, S> {
  pub fn create_render_object<SP: ShadingProvider<T>>(
    &mut self,
    geometry: GeometryHandle<T>,
    shading: ShadingHandle<T, SP>,
  ) -> RenderObjectHandle<T> {
    let obj = RenderObject::new(geometry, shading);
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
    self.render_objects.remove(index);
  }
}

pub struct Drawcall<T: RALBackend, S: SceneBackend<T>> {
  pub render_object: RenderObjectHandle<T>,
  pub node: SceneNodeHandle<T, S>,
}

impl<T: RALBackend, S: SceneBackend<T>> Clone for Drawcall<T, S> {
  fn clone(&self) -> Self {
    Self {
      render_object: self.render_object.clone(),
      node: self.node.clone(),
    }
  }
}

impl<T: RALBackend, S: SceneBackend<T>> Copy for Drawcall<T, S> {}
