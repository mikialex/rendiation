use crate::{ContentKey, RenderGraphBackend, RenderGraphNodeHandle};
use std::collections::HashMap;

pub struct ContentPool<T: RenderGraphBackend> {
  cached: Vec<T::ContentUnitImpl>,
  active_contents: HashMap<RenderGraphNodeHandle<T>, T::ContentUnitImpl>,
}

impl<T: RenderGraphBackend> ContentPool<T> {
  pub fn new() -> Self {
    Self {
      cached: Vec::new(),
      active_contents: HashMap::new(),
    }
  }

  pub fn load(&mut self, node: RenderGraphNodeHandle<T>) -> T::ContentUnitImpl {
    self.active_contents.remove(&node).unwrap()
  }

  pub fn store(&mut self, node: RenderGraphNodeHandle<T>, content: T::ContentUnitImpl) {
    self.active_contents.insert(node, content);
  }

  pub fn request_content(
    &mut self,
    node_handle: RenderGraphNodeHandle<T>,
  ) -> &mut T::ContentUnitImpl {
    let empty = if let Some(new) = self.cached.pop() {
      new
    } else {
      T::ContentUnitImpl::default()
    };

    self.active_contents.entry(node_handle).or_insert(empty)
  }

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
