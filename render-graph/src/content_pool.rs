use crate::{RenderGraphBackend, RenderGraphNodeHandle};
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

  pub fn active(&mut self, node: RenderGraphNodeHandle<T>) -> &mut T::ContentUnitImpl {
    let empty = if let Some(new) = self.cached.pop() {
      new
    } else {
      T::ContentUnitImpl::default()
    };

    self.active_contents.entry(node).or_insert(empty)
  }

  pub fn de_active(&mut self, node: RenderGraphNodeHandle<T>) {
    self
      .cached
      .push(self.active_contents.remove(&node).unwrap());
  }
}
