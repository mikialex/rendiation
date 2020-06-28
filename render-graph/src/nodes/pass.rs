use crate::{NodeBuilder, RenderGraphNode, RenderGraphNodeHandle, TargetNodeBuilder};
use rendiation_render_entity::Viewport;
use std::collections::HashSet;

pub struct PassNodeData {
  pub(crate) name: String,
  pub(crate) viewport: Viewport,
  pub(crate) input_targets_map: HashSet<RenderGraphNodeHandle>,
  pub(crate) render: Option<Box<dyn FnMut()>>,
}

pub struct PassNodeBuilder<'a> {
  pub(crate) builder: NodeBuilder<'a>,
}

impl<'a> PassNodeBuilder<'a> {
  pub fn handle(&self) -> RenderGraphNodeHandle {
    self.builder.handle
  }

  pub fn render_by(self, renderer: impl FnMut() + 'static) -> Self {
    self.pass_data_mut(|p| p.render = Some(Box::new(renderer)));
    self
  }

  pub fn pass_data_mut(&self, mutator: impl FnOnce(&mut PassNodeData)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data_handle = graph.get_node(self.handle()).data_handle();
    let data = graph.get_node_data_mut(data_handle);
    if let RenderGraphNode::Pass(data) = data {
      mutator(data)
    }
  }

  pub fn depend(self, target: &TargetNodeBuilder<'a>) -> Self {
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

  pub fn viewport(self) -> Self {
    self.pass_data_mut(|p| {
      // p.viewport
    });
    self
  }
}
