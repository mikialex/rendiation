use crate::{RenderGraphBackend, RenderGraphNodeHandle, TargetNodeData};
use std::collections::HashMap;

pub struct RenderTargetTypePooling<T: RenderGraphBackend> {
  key: T::RenderTargetFormatKey,
  available: Vec<T::RenderTarget>,
}

impl<T: RenderGraphBackend> RenderTargetTypePooling<T> {
  pub fn request(&mut self, renderer: &T::Renderer) -> T::RenderTarget {
    if self.available.len() == 0 {
      self
        .available
        .push(T::create_render_target(renderer, &self.key))
    }
    self.available.pop().unwrap()
  }

  pub fn return_back(&mut self, target: T::RenderTarget) {
    self.available.push(target)
  }
}

pub struct RenderTargetPool<T: RenderGraphBackend> {
  cached: HashMap<T::RenderTargetFormatKey, RenderTargetTypePooling<T>>,
  active_targets: HashMap<RenderGraphNodeHandle<T>, T::RenderTarget>,
}

impl<T: RenderGraphBackend> RenderTargetPool<T> {
  pub fn clear_all(&mut self, renderer: &T::Renderer) {
    if self.active_targets.len() > 0 {
      panic!("some target still in use")
    }
    self.cached.drain().for_each(|(_, p)|{
      p.available.into_iter().for_each(|t|{
        T::dispose_render_target(renderer, t)
      })
    })
  }

  fn get_pool(&mut self, key: &T::RenderTargetFormatKey) -> &mut RenderTargetTypePooling<T> {
    self
      .cached
      .entry(key.clone()) // todo, cost
      .or_insert_with(|| RenderTargetTypePooling {
        key: key.clone(),
        available: Vec::new(),
      })
  }

  /// get a RenderTarget from pool,if there is no fbo meet the config, create a new one, and pool it
  pub fn request_render_target(
    &mut self,
    node_handle: RenderGraphNodeHandle<T>,
    data: &TargetNodeData<T>,
    renderer: &T::Renderer,
  ) -> &T::RenderTarget {
    let target = self.get_pool(data.format()).request(renderer);
    self.active_targets.entry(node_handle).or_insert(target)
  }

  /// return a framebuffer that maybe request before, which will be pooling and reused
  pub fn return_render_target(
    &mut self,
    node_handle: RenderGraphNodeHandle<T>,
    data: &TargetNodeData<T>,
  ) {
    let target = self.active_targets.remove(&node_handle).unwrap();
    self.get_pool(data.format()).return_back(target);
  }
}
