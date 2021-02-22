#![allow(clippy::new_without_default)]
#![allow(clippy::type_complexity)]
pub use arena_graph::*;
pub use rendiation_algebra::*;
pub use rendiation_ral::*;
use std::{
  cell::{Cell, RefCell},
  hash::Hash,
};

mod backend;
mod content_pool;
mod executor;
mod nodes;
mod quad;
mod target_pool;
pub use backend::*;
pub use content_pool::*;
pub use executor::*;
pub use nodes::*;
pub use quad::*;
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
    result: &mut T::ContentUnitImpl,
  );
}

pub trait ContentUnit<T: RenderGraphGraphicsBackend, P>: Sized + Default {
  fn render_pass(&self, pass: &mut <T as RAL>::RenderPass, provider: &mut P);
}

pub type RenderGraphNodeHandle<T> = ArenaGraphNodeHandle<RenderGraphNode<T>>;

pub struct RenderGraphData<T: RenderGraphBackend> {
  graph: ArenaGraph<RenderGraphNode<T>>,
}

impl<T: RenderGraphBackend> RenderGraphData<T> {
  pub fn get_node_data_unwrap<U: FromRenderGraphNode<T>>(
    &self,
    node: RenderGraphNodeHandle<T>,
  ) -> &U {
    self.graph.get_node(node).data().downcast::<U>().unwrap()
  }
  pub fn get_node_data_mut_unwrap<U: FromRenderGraphNode<T>>(
    &mut self,
    node: RenderGraphNodeHandle<T>,
  ) -> &mut U {
    self
      .graph
      .get_node_mut(node)
      .data_mut()
      .downcast_mut::<U>()
      .unwrap()
  }
}

pub struct RenderGraph<T: RenderGraphBackend> {
  graph: RefCell<RenderGraphData<T>>,
  root_handle: Cell<Option<RenderGraphNodeHandle<T>>>,
  pass_queue: RefCell<Option<Vec<RenderGraphNodeHandle<T>>>>,
}

#[allow(clippy::new_without_default)]
impl<T: RenderGraphBackend> RenderGraph<T> {
  pub fn new() -> Self {
    Self {
      graph: RefCell::new(RenderGraphData {
        graph: ArenaGraph::new(),
      }),
      root_handle: Cell::new(None),
      pass_queue: RefCell::new(None),
    }
  }

  pub fn create_node(&self, data: RenderGraphNode<T>) -> RenderGraphNodeHandle<T> {
    self.graph.borrow_mut().graph.create_node(data)
  }
}

#[allow(clippy::needless_range_loop)]
fn build_pass_queue<T: RenderGraphBackend>(
  graph: &RenderGraph<T>,
) -> Vec<RenderGraphNodeHandle<T>> {
  let root = graph.root_handle.get().unwrap();
  let graph_data = &mut graph.graph.borrow_mut();
  let graph = &graph_data.graph;

  let node_list: Vec<RenderGraphNodeHandle<T>> = graph
    .topological_order_list(root)
    .unwrap()
    .into_iter()
    .collect();

  let mut target_drop_info = Vec::new();
  let mut content_drop_info = Vec::new();

  {
    let node_data_list: Vec<_> = node_list
      .iter()
      .map(|&n| (n, graph.get_node(n).data()))
      .collect();

    node_list
      .iter()
      .map(|&n| (n, graph.get_node(n).data()))
      .enumerate()
      .for_each(|(index, (n, data))| {
        use RenderGraphNode::*;
        if let Target(_) = data {
          for i in index.. {
            if let Pass(pass) = node_data_list[i].1 {
              if pass.input_targets_map.contains(&n) {
                target_drop_info.push((n, i));
                break;
              }
            }
          }
        };

        if data.is_content_node() {
          for i in index.. {
            if node_data_list[i].1.is_content_used_by(n) {
              content_drop_info.push((n, i));
              break;
            }
          }
        };
      });
  }

  target_drop_info.iter().for_each(|(n, i)| {
    graph_data
      .get_node_data_mut_unwrap::<PassNodeData<_>>(node_list[*i])
      .target_reuse_release_list
      .insert(*n);
  });

  content_drop_info.iter().for_each(|(n, i)| {
    graph_data
      .get_node_data_mut_unwrap::<PassNodeData<_>>(node_list[*i])
      .content_reuse_release_list
      .insert(*n);
  });

  node_list
}
