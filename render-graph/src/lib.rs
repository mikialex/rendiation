pub use arena_graph::*;
pub use rendiation_math::*;
pub use rendiation_ral::*;
use std::{
  cell::{Cell, RefCell},
  collections::HashSet,
  hash::Hash,
  marker::PhantomData,
};

mod backend;
mod content_pool;
mod executor;
mod nodes;
mod target_pool;
pub use backend::*;
pub use content_pool::*;
pub use executor::*;
pub use nodes::*;
pub use target_pool::*;

pub trait RenderGraphBackend: Sized {
  type Graphics: RenderGraphGraphicsBackend;
  type ContentProviderImpl: ContentProvider<Self>;
  type ContentSourceKey: Copy + Eq + Hash;
  type ContentMiddleKey: Copy + Eq + Hash;
  type ContentUnitImpl: ContentUnit<Self::Graphics, Self::ContentProviderImpl>;
}

pub trait ContentProvider<T: RenderGraphBackend> {
  fn get_source(
    &mut self,
    key: T::ContentSourceKey,
    pool: &RenderTargetPool<T>,
  ) -> T::ContentUnitImpl;
}

pub trait ContentUnit<T: RenderGraphGraphicsBackend, P>: Sized {
  fn render_pass(&self, pass: &mut <T as RALBackend>::RenderPass, provider: &mut P);
}

pub type RenderGraphNodeHandle<T> = ArenaGraphNodeHandle<RenderGraphNode<T>>;

pub struct RenderGraph<T: RenderGraphBackend> {
  graph: RefCell<ArenaGraph<RenderGraphNode<T>>>,
  root_handle: Cell<Option<RenderGraphNodeHandle<T>>>,
  pass_queue: RefCell<Option<Vec<PassExecuteInfo<T>>>>,
}

impl<T: RenderGraphBackend> RenderGraph<T> {
  pub fn new() -> Self {
    Self {
      graph: RefCell::new(ArenaGraph::new()),
      root_handle: Cell::new(None),
      pass_queue: RefCell::new(None),
    }
  }

  pub fn pass(&self, name: &str) -> PassNodeBuilder<T> {
    let handle = self
      .graph
      .borrow_mut()
      .create_node(RenderGraphNode::Pass(PassNodeData {
        name: name.to_owned(),
        viewport_modifier: Box::new(same_as_target),
        pass_op_modifier: Box::new(|b| b),
        input_targets_map: HashSet::new(),
        contents_to_render: Vec::new(),
      }));
    PassNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
        phantom: PhantomData,
      },
    }
  }

  pub fn finally(&self) -> TargetNodeBuilder<T> {
    let handle = self
      .graph
      .borrow_mut()
      .create_node(RenderGraphNode::Target(TargetNodeData::finally()));
    self.root_handle.set(Some(handle));

    TargetNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
        phantom: PhantomData,
      },
    }
  }

  pub fn source(&self, key: T::ContentSourceKey) -> ContentSourceNodeBuilder<T> {
    let handle = self
      .graph
      .borrow_mut()
      .create_node(RenderGraphNode::Source(ContentSourceNodeData { key }));

    ContentSourceNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
        phantom: PhantomData,
      },
    }
  }

  pub fn target(&self, name: &str) -> TargetNodeBuilder<T> {
    let handle =
      self
        .graph
        .borrow_mut()
        .create_node(RenderGraphNode::Target(TargetNodeData::target(
          name.to_owned(),
        )));

    TargetNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
        phantom: PhantomData,
      },
    }
  }
}

fn build_pass_queue<T: RenderGraphBackend>(graph: &RenderGraph<T>) -> Vec<PassExecuteInfo<T>> {
  let root = graph.root_handle.get().unwrap();
  let graph = graph.graph.borrow_mut();
  let node_list: Vec<RenderGraphNodeHandle<T>> = graph
    .topological_order_list(root)
    .unwrap()
    .into_iter()
    .filter(|&n| graph.get_node(n).data().is_pass())
    .collect();

  let mut exe_info_list: Vec<PassExecuteInfo<T>> = node_list
    .iter()
    .map(|&n| PassExecuteInfo {
      pass_node_handle: n,
      target_reuse_list: Vec::new(),
    })
    .collect();
  node_list.iter().enumerate().for_each(|(index, &n)| {
    let node = graph.get_node(n);
    let output_node = *node.to().iter().next().unwrap();
    let output_node_data = graph.get_node(output_node).data();
    if output_node_data
      .downcast::<TargetNodeData<_>>()
      .unwrap()
      .is_final_target()
    {
      return;
    }
    let mut last_used_index = node_list.len();
    node_list
      .iter()
      .enumerate()
      .skip(index)
      .rev()
      .take_while(|&(rev_index, n)| {
        let check_node = graph.get_node(*n);
        let result = check_node.from().contains(&output_node);
        if result {
          last_used_index = rev_index;
        }
        result
      })
      .for_each(|_| {});
    let list = &mut exe_info_list[last_used_index].target_reuse_list;
    if list.iter().position(|&x| x == output_node).is_none() {
      list.push(output_node)
    }
  });
  exe_info_list
}
