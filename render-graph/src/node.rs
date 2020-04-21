use crate::WrapNode;

pub enum GraphNodeType {
  Pass,
  Target
}

pub struct GraphData {
  index: usize,
  node_type: GraphNodeType,
}

pub struct TargetNode<GraphData> {
  node: WrapNode<GraphData>,
}

impl TargetNode<GraphData> {
  pub fn new() -> Self {
    todo!();
  }

  pub fn from(&mut self, node: PassNode) -> &mut Self {
    self
  }
}

pub struct PassNode {
  name: String,
  node: WrapNode<GraphData>,
  // clearColor:
}

impl PassNode {
  pub fn new(name: String, node: WrapNode<GraphData>) -> Self {
    Self { name, node }
  }

  pub fn draw() {}
}
