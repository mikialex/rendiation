use crate::WrapNode;
use std::{cell::RefCell, rc::Rc};

pub enum GraphNodeType {
  Pass,
  Target,
}

pub struct GraphData {
  pub(crate) index: usize,
  pub(crate) node_type: GraphNodeType,
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

pub struct PassNodeData {
  // clearColor:
}

pub struct PassNode {
  name: String,
  node: WrapNode<GraphData>,
  pass_info: Rc<RefCell<PassNodeData>>,
}

impl PassNode {
  pub fn new(
    name: String,
    node: WrapNode<GraphData>,
    pass_info: Rc<RefCell<PassNodeData>>,
  ) -> Self {
    Self {
      name,
      node,
      pass_info,
    }
  }

  pub fn draw() {}
}
