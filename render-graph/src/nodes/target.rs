use rendiation_ral::Viewport;

use crate::{
  NodeBuilder, PassNodeBuilder, RenderGraph, RenderGraphBackend, RenderGraphGraphicsBackend,
  RenderGraphNode, RenderTargetFormatKey, RenderTargetSize,
};

pub struct TargetNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, TargetNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> TargetNodeBuilder<'a, T> {
  pub fn draw_by_pass(self, pass: &PassNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&pass.builder);
    pass
      .builder
      .mutate_data(|p| p.target_to = Some(self.builder.handle));
    self
  }

  pub fn format(
    self,
    modifier: impl FnOnce(&mut <T::Graphics as RenderGraphGraphicsBackend>::RenderTargetFormatKey),
  ) -> Self {
    self.builder.mutate_data(|t| modifier(t.format_mut()));
    self
  }

  pub fn size_modifier(
    self,
    modifier: impl Fn(RenderTargetSize) -> RenderTargetSize + 'static,
  ) -> Self {
    self
      .builder
      .mutate_data(|t| t.size_modifier = Box::new(modifier));
    self
  }
}

impl<T: RenderGraphBackend> RenderGraph<T> {
  pub fn target(&self, name: &str) -> TargetNodeBuilder<T> {
    let handle = self.create_node(RenderGraphNode::Target(TargetNodeData::target(
      name.to_owned(),
    )));
    TargetNodeBuilder {
      builder: NodeBuilder::new(self, handle),
    }
  }

  pub fn finally(&self) -> TargetNodeBuilder<T> {
    let handle = self.create_node(RenderGraphNode::Target(TargetNodeData::finally()));
    TargetNodeBuilder {
      builder: NodeBuilder::new(self, handle),
    }
  }
}

pub struct TargetNodeData<T: RenderGraphBackend> {
  pub name: String,
  is_final_target: bool,
  size_modifier: Box<dyn Fn(RenderTargetSize) -> RenderTargetSize>,
  pub format:
    RenderTargetFormatKey<<T::Graphics as RenderGraphGraphicsBackend>::RenderTargetFormatKey>,
}

impl<T: RenderGraphBackend> TargetNodeData<T> {
  pub fn format(&self) -> &<T::Graphics as RenderGraphGraphicsBackend>::RenderTargetFormatKey {
    &self.format.format
  }
  pub fn format_mut(
    &mut self,
  ) -> &mut <T::Graphics as RenderGraphGraphicsBackend>::RenderTargetFormatKey {
    &mut self.format.format
  }

  pub fn update_real_size(&mut self, final_size: RenderTargetSize) -> RenderTargetSize {
    self.format.size = (self.size_modifier)(final_size);
    self.format.size
  }

  pub fn target(name: String) -> Self {
    Self {
      name,
      format: RenderTargetFormatKey::default_with_format(
        <T::Graphics as RenderGraphGraphicsBackend>::RenderTargetFormatKey::default(),
      ),
      is_final_target: false,
      size_modifier: Box::new(same_as_final),
    }
  }

  pub fn finally() -> Self {
    Self {
      name: "root".to_owned(),
      format: RenderTargetFormatKey::default_with_format(
        <T::Graphics as RenderGraphGraphicsBackend>::RenderTargetFormatKey::default(),
      ), // not actually useful
      is_final_target: true,
      size_modifier: Box::new(same_as_final),
    }
  }

  pub fn is_final_target(&self) -> bool {
    self.is_final_target
  }
}

pub fn same_as_target(size: RenderTargetSize) -> Viewport {
  Viewport::new(size.to_tuple())
}

pub fn same_as_final(size: RenderTargetSize) -> RenderTargetSize {
  size
}
