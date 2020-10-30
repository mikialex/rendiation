use crate::{
  default_impl::DefaultSceneBackend, DrawcallHandle, Scene, SceneBackend, SceneNodeHandle,
};
use rendiation_ral::*;

impl<T: RAL, S: SceneBackend<T>> Scene<T, S> {
  pub fn create_drawcall<SP: ShadingProvider<T>, G: GeometryProvider<T>>(
    &mut self,
    geometry: GeometryHandle<T, G>,
    shading: ShadingHandle<T, SP>,
  ) -> DrawcallHandle<T> {
    let obj = Drawcall::new_to_untyped(geometry, shading);
    self.drawcalls.insert(obj)
  }

  pub fn delete_drawcall(&mut self, index: DrawcallHandle<T>) {
    self.drawcalls.remove(index);
  }
}

pub struct SceneDrawcall<T: RAL, S: SceneBackend<T> = DefaultSceneBackend> {
  pub drawcall: DrawcallHandle<T>,
  pub node: SceneNodeHandle<T, S>,
}

impl<T: RAL, S: SceneBackend<T>> Clone for SceneDrawcall<T, S> {
  fn clone(&self) -> Self {
    Self {
      drawcall: self.drawcall.clone(),
      node: self.node.clone(),
    }
  }
}

impl<T: RAL, S: SceneBackend<T>> Copy for SceneDrawcall<T, S> {}

pub struct SceneDrawcallList<T: RAL, S: SceneBackend<T> = DefaultSceneBackend> {
  pub inner: Vec<SceneDrawcall<T, S>>,
}

impl<T: RAL, S: SceneBackend<T>> Default for SceneDrawcallList<T, S> {
  fn default() -> Self {
    SceneDrawcallList::new()
  }
}

impl<T: RAL, S: SceneBackend<T>> SceneDrawcallList<T, S> {
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
      let drawcall = scene.drawcalls.get(d.drawcall).unwrap();
      T::render_drawcall(&drawcall, pass, resources);
    })
  }
}

pub trait SceneRenderSource<T: RAL, S: SceneBackend<T>> {
  fn get_scene(&self) -> &Scene<T, S>;
  fn get_resource(&self) -> &ResourceManager<T>;
}

#[cfg(feature = "rendergraph")]
use rendiation_rendergraph::*;

#[cfg(feature = "rendergraph")]
impl<T: RenderGraphGraphicsBackend, S: SceneBackend<T>, U: SceneRenderSource<T, S>>
  ContentUnit<T, U> for SceneDrawcallList<T, S>
{
  fn render_pass(&self, pass: &mut T::RenderPass, provider: &mut U) {
    self.render(pass, provider.get_scene(), provider.get_resource())
  }
}
