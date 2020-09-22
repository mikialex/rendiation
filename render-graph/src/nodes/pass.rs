use crate::{
  ContentNodeBuilder, ContentProvider, NodeBuilder, RenderGraphBackend, RenderGraphNode,
  RenderGraphNodeHandle, RenderTargetSize, TargetNodeBuilder,
};
use rendiation_ral::Viewport;
use std::collections::HashSet;

pub struct PassNodeData<T: RenderGraphBackend, U: ContentProvider<T>> {
  pub name: String,
  pub(crate) viewport_modifier: Box<dyn Fn(RenderTargetSize) -> Viewport>,
  pub(crate) pass_op_modifier: Box<dyn FnMut(T::RenderPassBuilder) -> T::RenderPassBuilder>,
  pub(crate) input_targets_map: HashSet<RenderGraphNodeHandle<T, U>>,
  pub(crate) contents_to_render: Vec<RenderGraphNodeHandle<T, U>>,
}

impl<T: RenderGraphBackend, U: ContentProvider<T>> PassNodeData<T, U> {
  pub fn viewport(&mut self, target_size: RenderTargetSize) -> Viewport {
    (&self.viewport_modifier)(target_size)
  }
}

pub struct PassNodeBuilder<'a, T: RenderGraphBackend, U: ContentProvider<T>> {
  pub(crate) builder: NodeBuilder<'a, T, U>,
}

impl<'a, T: RenderGraphBackend, U: ContentProvider<T>> PassNodeBuilder<'a, T, U> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T, U> {
    self.builder.handle
  }

  pub fn define_pass_ops(
    self,
    modifier: impl FnMut(T::RenderPassBuilder) -> T::RenderPassBuilder + 'static,
  ) -> Self {
    self.pass_data_mut(|p| p.pass_op_modifier = Box::new(modifier));
    self
  }

  pub fn render_by(self, content: &ContentNodeBuilder<'a, T, U>) -> Self {
    self
      .builder
      .graph
      .graph
      .borrow_mut()
      .connect_node(content.handle(), self.handle());
    self.pass_data_mut(|p| {
      p.contents_to_render.push(content.handle());
    });
    self
  }

  pub fn viewport_modifier(
    self,
    modifier: impl Fn(RenderTargetSize) -> Viewport + 'static,
  ) -> Self {
    self.pass_data_mut(|p| p.viewport_modifier = Box::new(modifier));
    self
  }

  pub fn pass_data_mut(&self, mutator: impl FnOnce(&mut PassNodeData<T, U>)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data = graph.get_node_mut(self.handle()).data_mut();
    if let RenderGraphNode::Pass(data) = data {
      mutator(data)
    }
  }

  pub fn depend(self, target: &TargetNodeBuilder<'a, T, U>) -> Self {
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
