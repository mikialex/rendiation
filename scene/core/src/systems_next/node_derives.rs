use std::marker::PhantomData;

use futures::StreamExt;
use tree::*;

use crate::*;

type ReactiveParentTree =
  TreeHierarchyDerivedSystem<SceneNodeDerivedData, ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>>;

pub struct NodeIncrementalDeriveSystem {
  pub world_mat: Box<dyn DynamicReactiveCollection<(u64, usize), Mat4<f32>>>,
}

impl NodeIncrementalDeriveSystem {
  pub fn new(nodes: &SceneNodeCollection) -> Self {
    let stream = nodes.inner.source.batch_listen();
    let inner = TreeHierarchyDerivedSystem::<
      SceneNodeDerivedData,
      ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>,
    >::new::<ParentTree, _, _, _>(stream, &nodes.inner);

    let world_mat = TreeDeriveOutput {
      inner: inner.clone(),
      forked_change: Box::new(inner.derived_stream.fork_stream()),
      scene_id: nodes.scene_guid,
      downcast_delta: |d: SceneNodeDerivedDataDelta| match d {
        SceneNodeDerivedDataDelta::world_matrix(mat) => Some(mat),
        _ => None,
      },
      getter: |derive: &SceneNodeDerivedData| derive.world_matrix,
      phantom: PhantomData,
    };

    Self {
      world_mat: Box::new(world_mat),
    }
  }
}

pub struct TreeDeriveOutput<FD, F, V> {
  inner: ReactiveParentTree,
  forked_change:
    Box<dyn Stream<Item = Vec<CollectionDelta<usize, DeltaOf<SceneNodeDerivedData>>>> + Unpin>,
  scene_id: u64,
  downcast_delta: FD,
  getter: F,
  phantom: PhantomData<V>,
}

impl<FD, F, V> VirtualCollection<(u64, usize), V> for TreeDeriveOutput<FD, F, V>
where
  F: Fn(&SceneNodeDerivedData) -> V,
{
  fn iter_key(&self) -> impl Iterator<Item = (u64, usize)> + '_ {
    // todo, avoid clone by unsafe
    let tree = self.inner.derived_tree.read().unwrap();
    tree
      .iter_node_idx()
      .map(move |v| (self.scene_id, v))
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn access(&self) -> impl Fn(&(u64, usize)) -> Option<V> + '_ {
    let tree = self.inner.derived_tree.read().unwrap();
    move |(s_id, idx)| {
      if *s_id == self.scene_id {
        let handle = tree.try_recreate_handle(*idx)?;
        tree
          .try_get_node(handle)
          .map(|node| (self.getter)(&node.data().data))
      } else {
        None
      }
    }
  }
}

impl<FD, F, V> ReactiveCollection<(u64, usize), V> for TreeDeriveOutput<FD, F, V>
where
  V: Clone + 'static,
  F: Fn(&SceneNodeDerivedData) -> V + 'static,
  FD: Fn(SceneNodeDerivedDataDelta) -> Option<V> + 'static,
{
  type Changes = impl Iterator<Item = CollectionDelta<(u64, usize), V>> + Clone;

  fn poll_changes(
    &mut self,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Changes>> {
    // todo, should use loop poll and delta compact to maintain data coherency
    let changes = self.forked_change.poll_next_unpin(cx);
    let s_id = self.scene_id;
    changes.map(|v| {
      v.map(|v| {
        v.into_iter()
          .filter_map(|delta| match delta {
            CollectionDelta::Delta(idx, d, pd) => {
              let d = (self.downcast_delta)(d);
              let pd = pd.and_then(|pd| (self.downcast_delta)(pd));

              d.map(|d| CollectionDelta::Delta((s_id, idx), d, pd))
            }
            CollectionDelta::Remove(idx, d) => {
              (self.downcast_delta)(d).map(|mat| CollectionDelta::Remove((s_id, idx), mat))
            }
          })
          .collect::<Vec<_>>()
          .into_iter()
      })
    })
  }
}
