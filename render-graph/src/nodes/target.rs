use crate::{
  NodeBuilder, PassNodeBuilder, RenderGraph, RenderGraphBackend, RenderGraphNode,
  RenderGraphNodeHandle, RenderTargetFormatKey, RenderTargetSize,
};

pub struct TargetNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T>,
}

impl<'a, T: RenderGraphBackend> TargetNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }
  pub fn from_pass(self, passes: &PassNodeBuilder<'a, T>) -> Self {
    self
      .builder
      .graph
      .graph
      .borrow_mut()
      .connect_node(passes.handle(), self.handle());
    self
  }

  pub fn format(self, modifier: impl FnOnce(&mut T::RenderTargetFormatKey)) -> Self {
    self.target_data_mut(|t| modifier(t.format_mut()));
    self
  }

  pub fn size_modifier(
    self,
    modifier: impl Fn(RenderTargetSize) -> RenderTargetSize + 'static,
  ) -> Self {
    self.target_data_mut(|t| t.size_modifier = Box::new(modifier));
    self
  }

  pub fn target_data_mut(&self, mutator: impl FnOnce(&mut TargetNodeData<T>)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data = graph.get_node_mut(self.handle()).data_mut();
    if let RenderGraphNode::Target(data) = data {
      mutator(data)
    }
  }
}

pub struct TargetNodeData<T: RenderGraphBackend> {
  pub name: String,
  is_final_target: bool,
  size_modifier: Box<dyn Fn(RenderTargetSize) -> RenderTargetSize>,
  pub format: RenderTargetFormatKey<T::RenderTargetFormatKey>,
}

impl<T: RenderGraphBackend> TargetNodeData<T> {
  pub fn format(&self) -> &T::RenderTargetFormatKey {
    &self.format.format
  }
  pub fn format_mut(&mut self) -> &mut T::RenderTargetFormatKey {
    &mut self.format.format
  }

  pub fn real_size(&self) -> RenderTargetSize {
    self.format.size
  }

  pub fn update_real_size(&mut self, final_size: RenderTargetSize) -> &mut Self {
    self.format.size = (self.size_modifier)(final_size);
    self
  }

  pub fn target(name: String) -> Self {
    Self {
      name,
      format: RenderTargetFormatKey::default_with_format(T::RenderTargetFormatKey::default()),
      is_final_target: false,
      size_modifier: Box::new(RenderGraph::<T>::same_as_final),
    }
  }

  pub fn finally() -> Self {
    Self {
      name: "root".to_owned(),
      format: RenderTargetFormatKey::default_with_format(T::RenderTargetFormatKey::default()), // not actually useful
      is_final_target: true,
      size_modifier: Box::new(RenderGraph::<T>::same_as_final),
    }
  }

  pub fn is_final_target(&self) -> bool {
    self.is_final_target
  }
}
