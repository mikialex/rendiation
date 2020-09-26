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
  pass_queue: RefCell<Option<Vec<GraphExecutionInfo<T>>>>,
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

fn build_pass_queue<T: RenderGraphBackend>(graph: &RenderGraph<T>) -> Vec<GraphExecutionInfo<T>> {
  let root = graph.root_handle.get().unwrap();
  let graph = graph.graph.borrow_mut();

  let node_list: Vec<RenderGraphNodeHandle<T>> = graph
    .topological_order_list(root)
    .unwrap()
    .into_iter()
    .collect();

  let mut exe_info_list: Vec<_> = node_list
    .iter()
    .filter_map(|&n| graph.get_node(n).data().to_execution_info(n))
    .collect();

  node_list.iter().for_each(|&n| {
    let node = graph.get_node(n).data();
    let mut is_content = false;
    if let Some(_) = node.downcast::<ContentSourceNodeData<_>>() {
      is_content = true;
    }
    if let Some(_) = node.downcast::<ContentMiddleNodeData<_>>() {
      is_content = true;
    }

    if is_content {
      let mut reverse_count = 0;
      exe_info_list
        .iter_mut()
        .rev()
        .filter_map(|info| {
          if let GraphExecutionInfo::Pass(i) = info {
            Some(i)
          } else {
            None
          }
        })
        .take_while(|info| {
          let pass_info = graph
            .get_node(info.node)
            .data()
            .downcast::<PassNodeData<T>>()
            .unwrap();
          reverse_count += 1;
          !pass_info.contents_to_render.contains(&n)
        })
        .for_each(|_| {});
      let last_used_index = exe_info_list.len() - reverse_count;

      if let GraphExecutionInfo::Pass(i) = &mut exe_info_list[last_used_index] {
        i.content_reuse_release_list.insert(n);
      }
    }

    if let Some(_) = node.downcast::<TargetNodeData<_>>() {
      let mut reverse_count = 0;
      exe_info_list
        .iter_mut()
        .rev()
        .filter_map(|info| {
          if let GraphExecutionInfo::Pass(i) = info {
            Some(i)
          } else {
            None
          }
        })
        .take_while(|info| {
          let pass_info = graph
            .get_node(info.node)
            .data()
            .downcast::<PassNodeData<T>>()
            .unwrap();
          reverse_count += 1;
          !pass_info.input_targets_map.contains(&n)
        })
        .for_each(|_| {});
      let last_used_index = exe_info_list.len() - reverse_count;

      if let GraphExecutionInfo::Pass(i) = &mut exe_info_list[last_used_index] {
        i.target_reuse_release_list.insert(n);
      }
    }
  });

  exe_info_list
}
