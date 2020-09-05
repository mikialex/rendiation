use crate::{
  GeometryHandle, RALBackend, RenderObjectHandle, Scene, SceneBackend, SceneNodeHandle,
  ShadingHandle,
};
use rendiation_ral::{RenderObject, ResourceManager, ShadingProvider};

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

pub struct DrawcallList<T: RALBackend, S: SceneBackend<T>> {
  pub inner: Vec<Drawcall<T, S>>,
}

impl<T: RALBackend, S: SceneBackend<T>> DrawcallList<T, S> {
  pub fn new() -> Self {
    Self { inner: Vec::new() }
  }

  pub fn render(
    &self,
    pass: &mut T::RenderPass,
    scene: &Scene<T, S>,
    resources: &ResourceManager<T>,
  ) {
    self.inner.iter().for_each(|d| {
      let render_object = scene.render_objects.get(d.render_object).unwrap();
      T::render_object(&render_object, pass, resources);
    })
  }
}
