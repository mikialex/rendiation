use crate::{
  default_impl::DefaultSceneBackend, GeometryHandle, RALBackend, RenderObjectHandle, Scene,
  SceneBackend, SceneNodeHandle, ShadingHandle,
};
use rendiation_ral::{RenderObject, ResourceManager, ShadingProvider};

impl<T: RALBackend, S: SceneBackend<T>> Scene<T, S> {
  pub fn create_render_object<SP: ShadingProvider<T>, G: GeometryProvider<T>>(
    &mut self,
    geometry: GeometryHandle<T, G>,
    shading: ShadingHandle<T, SP>,
  ) -> RenderObjectHandle<T> {
    let obj = RenderObject::new(geometry, shading);
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
    self.render_objects.remove(index);
  }
}

pub struct Drawcall<T: RALBackend, S: SceneBackend<T> = DefaultSceneBackend> {
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

pub struct DrawcallList<T: RALBackend, S: SceneBackend<T> = DefaultSceneBackend> {
  pub inner: Vec<Drawcall<T, S>>,
}

impl<T: RALBackend, S: SceneBackend<T>> Default for DrawcallList<T, S> {
  fn default() -> Self {
    DrawcallList::new()
  }
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

pub trait SceneRenderSource<T: RALBackend, S: SceneBackend<T>> {
  fn get_scene(&self) -> &Scene<T, S>;
  fn get_resource(&self) -> &ResourceManager<T>;
}

#[cfg(feature = "rendergraph")]
use rendiation_rendergraph::*;

#[cfg(feature = "rendergraph")]
impl<T: RenderGraphGraphicsBackend, S: SceneBackend<T>, U: SceneRenderSource<T, S>>
  ContentUnit<T, U> for DrawcallList<T, S>
{
  fn render_pass(&self, pass: &mut T::RenderPass, provider: &mut U) {
    self.render(pass, provider.get_scene(), provider.get_resource())
  }
}
