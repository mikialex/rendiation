use crate::{
  ContentSourceNodeBuilder, NodeBuilder, RenderGraphBackend, RenderGraphGraphicsBackend,
  RenderGraphNodeHandle, RenderTargetSize, TargetNodeBuilder,
};
use rendiation_ral::Viewport;
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
  pub(crate) builder: NodeBuilder<'a, T, PassNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> PassNodeBuilder<'a, T> {
  pub fn define_pass_ops(
    self,
    modifier: impl Fn(
        <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
      ) -> <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder
      + 'static,
  ) -> Self {
    self
      .builder
      .mutate_data(|p| p.pass_op_modifier = Box::new(modifier));
    self
  }

  pub fn render_by(self, content: &ContentSourceNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&self.builder);
    self.builder.mutate_data(|p| {
      p.contents_to_render.push(content.handle());
    });
    self
  }

  pub fn viewport_modifier(
    self,
    modifier: impl Fn(RenderTargetSize) -> Viewport + 'static,
  ) -> Self {
    self
      .builder
      .mutate_data(|p| p.viewport_modifier = Box::new(modifier));
    self
  }

  pub fn depend(self, target: &TargetNodeBuilder<'a, T>) -> Self {
    self.builder.mutate_data(|p| {
      p.input_targets_map.insert(target.builder.handle);
    });
    self.builder.connect_from(&target.builder);
    self
  }
}
