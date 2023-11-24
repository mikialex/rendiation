use std::marker::PhantomData;

use futures::StreamExt;
use tree::*;

use crate::*;

type ReactiveParentTree =
  TreeHierarchyDerivedSystem<SceneNodeDerivedData, ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>>;

pub type NodeWorldMatrixGetter<'a> = &'a dyn Fn(&NodeIdentity) -> Option<Mat4<f32>>;
pub type NodeNetVisibleGetter<'a> = &'a dyn Fn(&NodeIdentity) -> Option<bool>;

#[derive(Clone)]
pub struct NodeIncrementalDeriveCollections {
  pub world_mat: RxCForker<NodeIdentity, Mat4<f32>>,
  pub net_visible: RxCForker<NodeIdentity, bool>,
}

impl NodeIncrementalDeriveCollections {
  pub fn world_matrixes_getter(&self) -> impl Fn(&NodeIdentity) -> Option<Mat4<f32>> + '_ {
    self.world_mat.access()
  }
  pub fn net_visible_getter(&self) -> impl Fn(&NodeIdentity) -> Option<bool> + '_ {
    self.net_visible.access()
  }

  pub fn filter_by_keysets(
    &self,
    range: impl ReactiveCollection<NodeIdentity, ()> + Clone,
  ) -> Self {
    let forked = self.clone();

    let world_mat = forked.world_mat.filter_by_keyset(range.clone());
    let net_visible = forked.net_visible.filter_by_keyset(range);

    let world_mat = Box::new(world_mat) as Box<dyn DynamicReactiveCollection<_, _>>;
    let net_visible = Box::new(net_visible) as Box<dyn DynamicReactiveCollection<_, _>>;

    Self {
      world_mat: world_mat.into_forker(),
      net_visible: net_visible.into_forker(),
    }
  }
}

impl NodeIncrementalDeriveCollections {
  pub fn new(nodes: &SceneNodeCollection) -> Self {
    let stream = nodes.inner.source.batch_listen();
    let inner = TreeHierarchyDerivedSystem::<
      SceneNodeDerivedData,
      ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>,
    >::new::<ParentTree, _, _, _>(stream, &nodes.inner);

    let world_mat = TreeDeriveOutput::new(
      &inner,
      nodes.scene_guid,
      |d| match d {
        SceneNodeDerivedDataDelta::world_matrix(v) => Some(v),
        _ => None,
      },
      |derive: &SceneNodeDerivedData| derive.world_matrix,
    );

    let net_visible = TreeDeriveOutput::new(
      &inner,
      nodes.scene_guid,
      |d| match d {
        SceneNodeDerivedDataDelta::net_visible(v) => Some(v),
        _ => None,
      },
      |derive: &SceneNodeDerivedData| derive.net_visible,
    );

    let world_mat = Box::new(world_mat) as Box<dyn DynamicReactiveCollection<_, _>>;
    let net_visible = Box::new(net_visible) as Box<dyn DynamicReactiveCollection<_, _>>;

    Self {
      world_mat: world_mat.into_forker(),
      net_visible: net_visible.into_forker(),
    }
  }
}

pub struct TreeDeriveOutput<FD, F, V> {
  inner: ReactiveParentTree,
  forked_change: Box<
    dyn Stream<Item = Vec<CollectionDelta<usize, DeltaOf<SceneNodeDerivedData>>>>
      + Unpin
      + Send
      + Sync,
  >,
  scene_id: u64,
  downcast_delta: FD,
  getter: F,
  phantom: PhantomData<V>,
}

impl<FD, F, V> TreeDeriveOutput<FD, F, V> {
  pub fn new(inner: &ReactiveParentTree, scene_id: u64, downcast_delta: FD, getter: F) -> Self {
    let forked_change = Box::new(inner.derived_stream.fork_stream());
    Self {
      inner: inner.clone(),
      forked_change,
      scene_id,
      downcast_delta,
      getter,
      phantom: Default::default(),
    }
  }
}

impl<FD, F, V> VirtualCollection<NodeIdentity, V> for TreeDeriveOutput<FD, F, V>
where
  F: Fn(&SceneNodeDerivedData) -> V,
  FD: Sync,
  F: Sync,
  V: Sync,
{
  fn iter_key(&self) -> impl Iterator<Item = NodeIdentity> + '_ {
    // todo, avoid clone by unsafe
    let tree = self.inner.derived_tree.read().unwrap();
    tree
      .iter_node_idx()
      .map(move |v| (self.scene_id, v))
      .collect::<Vec<_>>()
      .into_iter()
  }

  fn access(&self) -> impl Fn(&NodeIdentity) -> Option<V> + Sync + '_ {
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

impl<FD, F, V> ReactiveCollection<NodeIdentity, V> for TreeDeriveOutput<FD, F, V>
where
  V: Clone + Send + Sync + 'static,
  F: Fn(&SceneNodeDerivedData) -> V + Send + Sync + 'static,
  FD: Fn(SceneNodeDerivedDataDelta) -> Option<V> + Send + Sync + 'static,
{
  type Changes = impl CollectionChanges<NodeIdentity, V>;

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
          .into_par_iter()
      })
    })
  }

  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}
}
