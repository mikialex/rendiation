pub use arena_graph::*;

pub fn build_graph() {
  let mut graph = RenderGraph::new();
  let normal_pass = graph.pass("normal");
  let normal_target = graph.target("normal");

  // let pass = graph.pass("scene").useQuad();
  // RenderGraph::new().root().from_pass(pass)
}


pub enum RenderGraphNode{
  PassNode(PassNodeData),
  Target(TargetNodeData)
}

pub struct PassNodeData {
  name: String
}

pub struct TargetNodeData{
  name: String,
}

pub struct NodeBuilder<'a> {
  handle: ArenaGraphNodeHandle<RenderGraphNode>,
  graph: &'a mut RenderGraph,
}

pub struct RenderGraph {
  graph: ArenaGraph<RenderGraphNode>
}

impl RenderGraph {
  pub fn new() -> Self {
    Self {
      graph: ArenaGraph::new(),
    }
  }

  pub fn build(&mut self) {}

  pub fn render() {}

  pub fn pass(&mut self, name: &str) -> NodeBuilder {
    todo!()
  }

  pub fn target(&mut self, name: &str) -> NodeBuilder {
    todo!()
  }
}
