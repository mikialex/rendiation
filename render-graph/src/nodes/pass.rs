use crate::{
  NodeBuilder, RenderGraphBackend, RenderGraphNode, RenderGraphNodeHandle, RenderTargetPool,
  RenderTargetSize, TargetNodeBuilder,
};
use rendiation_render_entity::Viewport;
use std::collections::HashSet;

pub struct PassNodeData<T: RenderGraphBackend> {
  pub name: String,
  pub(crate) viewport_creator: Box<dyn Fn(RenderTargetSize) -> Viewport>,
  pub(crate) input_targets_map: HashSet<RenderGraphNodeHandle<T>>,
  pub(crate) render: Option<Box<dyn FnMut(&RenderTargetPool<T>, &mut T::RenderPass)>>,
}

impl<T: RenderGraphBackend> PassNodeData<T> {
  pub fn viewport(&mut self, target_size: RenderTargetSize) -> Viewport {
    (&self.viewport_creator)(target_size)
  }
}

pub struct PassNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T>,
}

impl<'a, T: RenderGraphBackend> PassNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }

  pub fn render_by(
    self,
    renderer: impl FnMut(&RenderTargetPool<T>, &mut T::RenderPass) + 'static,
  ) -> Self {
    self.pass_data_mut(|p| p.render = Some(Box::new(renderer)));
    self
  }

  pub fn viewport_creator(self, creator: impl Fn(RenderTargetSize) -> Viewport + 'static) -> Self {
    self.pass_data_mut(|p| p.viewport_creator = Box::new(creator));
    self
  }

  pub fn pass_data_mut(&self, mutator: impl FnOnce(&mut PassNodeData<T>)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data_handle = graph.get_node(self.handle()).data_handle();
    let data = graph.get_node_data_mut(data_handle);
    if let RenderGraphNode::Pass(data) = data {
      mutator(data)
    }
  }

  pub fn depend(self, target: &TargetNodeBuilder<'a, T>) -> Self {
    self.pass_data_mut(|p| {
      p.input_targets_map.insert(target.handle());
    });
    self
      .builder
      .graph
      .graph
      .borrow_mut()
      .connect_node(target.handle(), self.handle());
    self
  }
}
