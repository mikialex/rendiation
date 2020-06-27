pub use arena_graph::*;
pub use rendiation_math::*;
pub use rendiation_render_entity::*;
use std::{
  cell::{Cell, RefCell},
  collections::HashSet,
};

mod nodes;
pub use nodes::*;

pub type RenderGraphNodeHandle = ArenaGraphNodeHandle<RenderGraphNode>;

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
        viewport: Viewport::new((1, 1)),
        input_targets_map: HashSet::new(),
        render: None,
      }));
    PassNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
      },
    }
  }

  pub fn screen(&self, target: &PassNodeBuilder) {
    self.root_handle.set(Some(target.handle()));
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
  let copy_screen = graph
    .pass("copy_screen")
    .viewport()
    .depend(&normal_target);
  graph.screen(&copy_screen);
  // let pass = graph.pass("scene").useQuad();
  // RenderGraph::new().root().from_pass(pass)
}
