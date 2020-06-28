use crate::{NodeBuilder, PassNodeBuilder, RenderGraphNodeHandle};

pub struct TargetNodeBuilder<'a> {
  pub(crate) builder: NodeBuilder<'a>,
}

impl<'a> TargetNodeBuilder<'a> {
  pub fn handle(&self) -> RenderGraphNodeHandle {
    self.builder.handle
  }
  pub fn from_pass(self, passes: &PassNodeBuilder<'a>) -> Self {
    self
  }
}

pub struct TargetNodeData {
  pub name: String,
  is_screen: bool,
}

impl TargetNodeData {
  pub fn target(name: String) -> Self {
    Self {
      name,
      is_screen: false,
    }
  }
  pub fn screen() -> Self {
    Self {
      name: "root".to_owned(),
      is_screen: true,
    }
  }
  pub fn is_screen(&self) -> bool {
    self.is_screen
  }
}
