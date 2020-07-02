use crate::{
  NodeBuilder, PassNodeBuilder, RenderGraphBackend, RenderGraphNode, RenderGraphNodeHandle,
  RenderTargetFormatKey,
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

  pub fn target_data_mut(&self, mutator: impl FnOnce(&mut TargetNodeData<T>)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data_handle = graph.get_node(self.handle()).data_handle();
    let data = graph.get_node_data_mut(data_handle);
    if let RenderGraphNode::Target(data) = data {
      mutator(data)
    }
  }
}

pub struct TargetNodeData<T: RenderGraphBackend> {
  pub name: String,
  is_final_target: bool,
  pub format: RenderTargetFormatKey<T::RenderTargetFormatKey>,
}

impl<T: RenderGraphBackend> TargetNodeData<T> {
  pub fn format(&self) -> &T::RenderTargetFormatKey {
    &self.format.format
  }
  pub fn format_mut(&mut self) -> &mut T::RenderTargetFormatKey {
    &mut self.format.format
  }

  pub fn target(name: String) -> Self {
    Self {
      name,
      format: RenderTargetFormatKey::default_with_format(T::RenderTargetFormatKey::default()),
      is_final_target: false,
    }
  }
  pub fn finally() -> Self {
    Self {
      name: "root".to_owned(),
      format: RenderTargetFormatKey::default_with_format(T::RenderTargetFormatKey::default()), // not actually useful
      is_final_target: true,
    }
  }
  pub fn is_final_target(&self) -> bool {
    self.is_final_target
  }
}
