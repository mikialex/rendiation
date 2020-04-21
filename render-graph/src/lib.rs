pub mod graph;
pub mod node;
pub use graph::*;
pub use node::*;
use std::{cell::RefCell, rc::Rc};

pub struct RenderGraph {
  graph: Graph<GraphData>,
  passes: Vec<usize>,
  pass_nodes: Vec<Rc<RefCell<PassNodeData>>>,
}

impl RenderGraph {
  pub fn new() -> Self {
    Self {
      graph: Graph::new(),
      passes: Vec::new(),
      pass_nodes: Vec::new(),
    }
  }

  pub fn build(&mut self) {}

  pub fn render() {}

  pub fn pass(&mut self, name: &str) -> PassNode {
    let pass_data = PassNodeData {};
    let pass_data_wrap = Rc::new(RefCell::new(pass_data));
    let index = self.pass_nodes.len();
    self.pass_nodes.push(pass_data_wrap);

    let data = GraphData {
      index,
      node_type: GraphNodeType::Pass,
    };

    let wrap_node = self.graph.create_node(data);
    let node = PassNode::new(name.into(), wrap_node, pass_data_wrap);
    node
  }

  pub fn target() {}
}
