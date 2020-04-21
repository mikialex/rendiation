pub mod graph;
pub mod node;
pub use graph::*;
pub use node::*;

pub struct RenderGraph {
  graph: Graph<GraphData>,
  passes: Vec<usize>,
}

impl RenderGraph {
  pub fn new() -> Self {
    Self {
      graph: Graph::new(),
      passes: Vec::new(),
    }
  }

  pub fn build(&mut self) {}

  pub fn render() {}

  pub fn pass(&mut self, name: &str) -> PassNode {
    let wrap_node = self.graph.create_node();
    let node = PassNode::new(name.into(), wrap_node);
    node
  }

  pub fn target(){

  }
}

