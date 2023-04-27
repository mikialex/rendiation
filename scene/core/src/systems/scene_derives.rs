use crate::*;
use futures::Stream;
use reactive::*;
use tree::CoreTree;
use tree::ParentTree;
use tree::ParentTreeDirty;
use tree::TreeHierarchyDerivedSystem;

#[derive(Clone)]
pub struct SceneNodeDeriveSystem {
  pub(crate) inner:
    TreeHierarchyDerivedSystem<SceneNodeDerivedData, ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>>,
  pub(crate) indexed_stream_mapper: SceneNodeChangeStreamIndexMapper,
}
type SceneNodeChangeStream = impl Stream<Item = (usize, SceneNodeDerivedDataDelta)> + Unpin;

pub type SceneNodeChangeStreamIndexMapper =
  StreamBroadcaster<SceneNodeChangeStream, SceneNodeDerivedDataDelta, IndexMapping>;

impl SceneNodeDeriveSystem {
  pub fn new(nodes: &SceneNodeCollection) -> Self {
    let inner_sys = nodes.inner.visit_inner(|tree| {
      let stream = tree.source.listen();
      TreeHierarchyDerivedSystem::<
        SceneNodeDerivedData,
        ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>,
      >::new::<ParentTree, _, _, _>(stream, &nodes.inner)
    });

    let indexed_stream_mapper: SceneNodeChangeStreamIndexMapper = inner_sys
      .derived_stream
      .fork_stream()
      .create_index_mapping_broadcaster();

    Self {
      inner: inner_sys,
      indexed_stream_mapper,
    }
  }

  pub fn maintain(&mut self) {
    do_updates(&mut self.indexed_stream_mapper, |_| {});
  }
}

impl SceneNodeDeriveSystem {
  pub fn get_world_matrix(&self, node: &SceneNode) -> Mat4<f32> {
    self.get_world_matrix_by_raw_handle(node.raw_handle().index())
  }

  pub fn get_world_matrix_by_raw_handle(&self, index: usize) -> Mat4<f32> {
    self.inner.visit_derived_tree(|tree| {
      let handle = tree.recreate_handle(index);
      tree.get_node(handle).data().data.world_matrix
    })
  }
  pub fn create_world_matrix_stream(
    &self,
    node: &SceneNode,
  ) -> impl Stream<Item = Mat4<f32>> + 'static {
    self
      .indexed_stream_mapper
      .create_sub_stream_by_index(node.raw_handle().index())
      .filter_map_sync(|d| match d {
        SceneNodeDerivedDataDelta::world_matrix(m) => Some(m),
        SceneNodeDerivedDataDelta::net_visible(_) => None,
      })
  }
  pub fn get_net_visible(&self, node: &SceneNode) -> bool {
    self.inner.visit_derived_tree(|tree| {
      let handle = tree.recreate_handle(node.raw_handle().index());
      tree.get_node(handle).data().data.net_visible
    })
  }
}
