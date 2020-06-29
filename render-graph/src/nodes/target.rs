use crate::{NodeBuilder, PassNodeBuilder, RenderGraphNodeHandle, RenderGraphBackend};

pub struct TargetNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T>,
}

impl<'a, T: RenderGraphBackend> TargetNodeBuilder<'a, T> {
  pub fn handle(&self) -> RenderGraphNodeHandle<T> {
    self.builder.handle
  }
  pub fn from_pass(self, passes: &PassNodeBuilder<'a, T>) -> Self {
    self
  }
}

pub struct TargetNodeData<T: RenderGraphBackend> {
  pub name: String,
  is_screen: bool,
  format: T::RenderTargetFormatKey
}

impl<T: RenderGraphBackend> TargetNodeData<T> {
  pub fn format(&self) ->  &T::RenderTargetFormatKey {
    &self.format
  }

  pub fn target(name: String) -> Self {
    Self {
      name,
      format: T::RenderTargetFormatKey::default(),
      is_screen: false,
    }
  }
  pub fn screen() -> Self {
    Self {
      name: "root".to_owned(),
      format: T::RenderTargetFormatKey::default(), // not actully useful
      is_screen: true,
    }
  }
  pub fn is_screen(&self) -> bool {
    self.is_screen
  }
}
