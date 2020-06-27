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

impl<'a> TargetNodeBuilder<'a> {
  pub fn from_pass(self, passes: &PassNodeBuilder<'a>) -> Self {
    self
  }
}

pub struct PassNodeBuilder<'a> {
  builder: NodeBuilder<'a>,
}

impl<'a> PassNodeBuilder<'a> {
  pub fn pass_data_mut(&self, mutator: impl FnOnce(&mut PassNodeData)) {
    let mut graph = self.builder.graph.graph.borrow_mut();
    let data_handle = graph.get_node(self.builder.handle).data_handle();
    let data = graph.get_node_data_mut(data_handle);
    if let RenderGraphNode::Pass(data) = data {
      mutator(data)
    }
  }

  pub fn input(self, name: &str ,target: &TargetNodeBuilder<'a>) -> Self {
    self
  }

  pub fn viewport(self) -> Self {
    self.pass_data_mut(|p| {
      // p.viewport
    });
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

  pub fn screen(&self, target: &PassNodeBuilder) {
    self.root_handle.set(Some(target.builder.handle));
  }

  pub fn target(&self, name: &str) -> TargetNodeBuilder {
    let handle = self
      .graph
      .borrow_mut()
      .new_node(RenderGraphNode::Target(TargetNodeData {
        name: name.to_owned(),
      }));
    TargetNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
      },
    }
  }
}

pub fn build_test_graph() {
  let graph = RenderGraph::new();
  let normal_pass = graph.pass("normal").viewport();
  let normal_target = graph.target("normal").from_pass(&normal_pass);
  let copy_screen = graph.pass("copy_screen").viewport().input("from", &normal_target);
  graph.screen(&copy_screen);
  // let pass = graph.pass("scene").useQuad();
  // RenderGraph::new().root().from_pass(pass)
}
