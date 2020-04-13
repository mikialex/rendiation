pub mod graph;


#[derive(Default)]
pub struct RenderGraph {
  nodes: Vec<RenderGraphNode>,
}

impl RenderGraph {
  pub fn new() -> Self {
    let mut graph = Self {
      nodes: Vec::new(),
    };
    let root = TargetNode::new();
    graph.nodes.push(RenderGraphNode::Target(root));
    graph
  }

  pub fn build(&mut self) {}

  pub fn reset(&mut self) -> &mut Self {
    self.nodes.clear();
    let root = TargetNode::new();
    self.nodes.push(RenderGraphNode::Target(root));
    self
  }

  pub fn get_root(&mut self) -> &mut TargetNode {
    if let RenderGraphNode::Target(target) = &mut self.nodes[0] {
      target
    } else {
      panic!();
    }
  }

  // pub fn pass(&mut self) -> &mut PassNode {
  //     // let node = PassNode::new();
  //     // node
  // }
}

pub enum RenderGraphNode {
  Pass(PassNode),
  Target(TargetNode),
}

impl RenderGraphNode {
  // pub fn
}

#[derive(Default)]
pub struct PassNode {
  name: String,
  from_target_id: Vec<usize>,
  to_target_id: Vec<usize>,
}

impl PassNode {
  pub fn new() -> Self {
    todo!();
  }

  pub fn draw() {}
}

// pub fn pass(name: &'static str) -> PassNode {

// }

pub struct TargetNode {
  name: String,
  from_pass_id: Vec<usize>,
  to_pass_id: Vec<usize>,
}

impl TargetNode {
  pub fn new() -> Self {
    todo!();
  }

  pub fn from(&mut self, node: TargetNode) -> &mut Self {
    self
  }
}
