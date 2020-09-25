use crate::{RenderGraphBackend, RenderGraphNodeHandle};
use std::collections::HashMap;

pub struct ContentPool<T: RenderGraphBackend> {
  cached: HashMap<T::ContentMiddleKey, T::ContentUnitImpl>,
  active_contents: HashMap<RenderGraphNodeHandle<T>, T::ContentUnitImpl>,
}

impl<T: RenderGraphBackend> ContentPool<T> {
  pub fn new() -> Self {
    Self {
      cached: HashMap::new(),
      active_contents: HashMap::new(),
    }
  }

  //   /// get a content container from pool,if there is no fbo meet the config, create a new one, and pool it
  //   pub fn request_render_target(
  //     &mut self,
  //     node_handle: RenderGraphNodeHandle<T>,
  //     data: &TargetNodeData<T>,
  //     renderer: &<T::Graphics as RALBackend>::Renderer,
  //   ) -> &<T::Graphics as RALBackend>::RenderTarget {
  //     let target = self.get_pool(&data.format).request(renderer);
  //     self.active_targets.entry(node_handle).or_insert(target)
  //   }

  //   /// return a content container that maybe request before, which will be pooling and reused
  //   pub fn return_render_target(
  //     &mut self,
  //     node_handle: RenderGraphNodeHandle<T>,
  //     data: &TargetNodeData<T>,
  //   ) {
  //     let target = self.active_targets.remove(&node_handle).unwrap();
  //     self.get_pool(&data.format).return_back(target);
  //   }
}
