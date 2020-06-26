pub use arena_graph::*;
pub use rendiation_math::*;
use std::cell::{Cell, RefCell};

pub type RenderGraphNodeHandle = ArenaGraphNodeHandle<RenderGraphNode>;
pub enum RenderGraphNode {
  Pass(PassNodeData),
  Target(TargetNodeData),
}

pub struct PassNodeData {
  pub name: String,
  viewport: Vec4<f32>,
}

pub struct TargetNodeData {
  pub name: String,
}

pub struct NodeBuilder<'a> {
  handle: RenderGraphNodeHandle,
  graph: &'a RenderGraph,
}

pub struct TargetNodeBuilder<'a> {
  builder: NodeBuilder<'a>,
}

pub struct PassNodeBuilder<'a> {
  builder: NodeBuilder<'a>,
}

impl<'a> PassNodeBuilder<'a> {
  pub fn viewport(self) -> Self {
    self
  }
}

pub struct RenderGraph {
  graph: RefCell<ArenaGraph<RenderGraphNode>>,
  root_handle: Cell<Option<RenderGraphNodeHandle>>,
}

impl RenderGraph {
  pub fn new() -> Self {
    Self {
      graph: RefCell::new(ArenaGraph::new()),
      root_handle: Cell::new(None),
    }
  }

  pub fn build(&mut self) {}

  pub fn render() {}

  pub fn pass(&self, name: &str) -> PassNodeBuilder {
    let handle = self
      .graph
      .borrow_mut()
      .new_node(RenderGraphNode::Pass(PassNodeData {
        name: name.to_owned(),
        viewport: Vec4::zero(),
      }));
    PassNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
      },
    }
  }

  pub fn screen(&self, target: PassNodeBuilder) {
    self.root_handle.set(Some(target.builder.handle));
  }

  pub fn target(&self, name: &str) -> TargetNodeBuilder {
    todo!()
  }
}

pub fn build_test_graph() {
  let mut graph = RenderGraph::new();
  let normal_pass = graph.pass("normal").viewport();
  let normal_target = graph.target("normal");
  let copy_screen = graph.pass("copy_screen").viewport();
  graph.screen(copy_screen);
  // let pass = graph.pass("scene").useQuad();
  // RenderGraph::new().root().from_pass(pass)
}