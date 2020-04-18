

pub mod graph;
pub mod node;
pub use node::*;
pub use graph::*;

pub struct RenderGraph {
  graph: Graph,
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

  pub fn pass(&mut self) {
  }
}

// pub enum RenderGraphNode {
//   Pass(PassNode),
//   Target(TargetNode),
// }
