use crate::{
  ContentNodeBuilder, ContentProvider, NodeBuilder, RenderGraph, RenderGraphBackend,
  RenderGraphGraphicsBackend, RenderGraphNode, RenderGraphNodeHandle, RenderTargetSize,
  TargetNodeBuilder,
};
use rendiation_ral::{RALBackend, Viewport};
use std::collections::HashSet;

pub struct PassNodeData<T: RenderGraphBackend> {
  pub name: String,
  pub(crate) viewport_modifier: Box<dyn Fn(RenderTargetSize) -> Viewport>,
  pub(crate) pass_op_modifier: Box<
    dyn Fn(
      <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
    ) -> <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
  >,
  pub(crate) input_targets_map: HashSet<RenderGraphNodeHandle<T>>,
  pub(crate) contents_to_render: Vec<RenderGraphNodeHandle<T>>,
}

impl<T: RenderGraphBackend> PassNodeData<T> {
  pub fn viewport(&self, target_size: RenderTargetSize) -> Viewport {
    (&self.viewport_modifier)(target_size)
  }
}

pub struct PassNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T>,
}

impl<'a, T: RenderGraphBackend> PassNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }

  pub fn define_pass_ops(
    self,
    modifier: impl Fn(
        <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
      ) -> <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder
      + 'static,
  ) -> Self {
    self.pass_data_mut(|p| p.pass_op_modifier = Box::new(modifier));
    self
  }

  pub fn render_by(self, content: &ContentNodeBuilder<'a, T>) -> Self {
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

  pub fn pass_data_mut(&self, mutator: impl FnOnce(&mut PassNodeData<T>)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data = graph.get_node_mut(self.handle()).data_mut();
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
