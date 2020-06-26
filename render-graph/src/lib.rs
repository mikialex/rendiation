pub use arena_graph::*;
pub use rendiation_math::*;


pub fn build_graph() {
  let mut graph = RenderGraph::new();
  let normal_pass = graph.pass("normal").viewport();
  let normal_target = graph.target("normal");
  graph.screen(normal_pass);
  // let pass = graph.pass("scene").useQuad();
  // RenderGraph::new().root().from_pass(pass)
}

pub type RenderGraphNodeHandle = ArenaGraphNodeHandle<RenderGraphNode>;
pub enum RenderGraphNode{
  Pass(PassNodeData),
  Target(TargetNodeData)
}

pub struct PassNodeData {
  pub name: String,
  viewport: Vec4<f32>
}

pub struct TargetNodeData{
  pub name: String,
}

pub struct NodeBuilder<'a> {
  handle: RenderGraphNodeHandle,
  graph: &'a mut RenderGraph,
}

pub struct TargetNodeBuilder<'a>{
  builder: NodeBuilder<'a>
}

pub struct PassNodeBuilder<'a>{
  builder: NodeBuilder<'a>
}

impl<'a> PassNodeBuilder<'a> {
  pub fn viewport(mut self) -> Self {
    self
  }
}

pub struct RenderGraph {
  graph: ArenaGraph<RenderGraphNode>,
  root_handle: Option<RenderGraphNodeHandle>
}

impl RenderGraph {
  pub fn new() -> Self {
    Self {
      graph: ArenaGraph::new(),
      root_handle: None
    }
  }

  pub fn build(&mut self) {}

  pub fn render() {}

  pub fn pass(&mut self, name: &str) -> PassNodeBuilder {
    let handle = self.graph.new_node(RenderGraphNode::Pass(
      PassNodeData {
        name: name.to_owned(),
        viewport: Vec4::zero()
      }
    ));
    PassNodeBuilder{
      builder: NodeBuilder{
        handle,
        graph: self,
      }
    }
  }

  pub fn screen(&mut self, target: PassNodeBuilder) {
    self.root_handle = Some(target.builder.handle);
  }

  pub fn target(&mut self, name: &str) -> TargetNodeBuilder {
    todo!()
  }
}
