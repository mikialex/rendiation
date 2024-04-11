use std::{marker::PhantomData, task::Poll};

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
    self.world_mat.make_accessor()
  }
  pub fn net_visible_getter(&self) -> impl Fn(&NodeIdentity) -> Option<bool> + '_ {
    self.net_visible.make_accessor()
  }

  pub fn filter_by_keysets(
    &self,
    range: impl ReactiveCollection<NodeIdentity, ()> + Clone,
  ) -> Self {
    let forked = self.clone();

    let world_mat = forked.world_mat.filter_by_keyset(range.clone());
    let net_visible = forked.net_visible.filter_by_keyset(range);

    let world_mat = Box::new(world_mat) as Box<dyn ReactiveCollection<_, _>>;
    let net_visible = Box::new(net_visible) as Box<dyn ReactiveCollection<_, _>>;

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

    let world_mat = Box::new(world_mat) as Box<dyn ReactiveCollection<_, _>>;
    let net_visible = Box::new(net_visible) as Box<dyn ReactiveCollection<_, _>>;

    Self {
      world_mat: world_mat.into_forker(),
      net_visible: net_visible.into_forker(),
    }
  }
}

pub struct TreeDeriveOutput<FD, F, V> {
  inner: ReactiveParentTree,
  forked_change: RwLock<
    Box<
      dyn Stream<Item = Vec<(usize, ValueChange<DeltaOf<SceneNodeDerivedData>>)>>
        + Unpin
        + Send
        + Sync,
    >,
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
      forked_change: RwLock::new(forked_change),
      scene_id,
      downcast_delta,
      getter,
      phantom: Default::default(),
    }
  }
}

impl<FD, F, V> ReactiveCollection<NodeIdentity, V> for TreeDeriveOutput<FD, F, V>
where
  V: CValue,
  F: Fn(&SceneNodeDerivedData) -> V + Send + Sync + 'static,
  FD: Fn(SceneNodeDerivedDataDelta) -> Option<V> + Send + Sync + 'static,
{
  fn poll_changes(
    &self,
    cx: &mut std::task::Context<'_>,
  ) -> PollCollectionChanges<NodeIdentity, V> {
    // todo, should use loop poll and delta compact to maintain data coherency
    let changes = self.forked_change.write().unwrap().poll_next_unpin(cx);
    let s_id = self.scene_id;
    match changes {
      std::task::Poll::Ready(Some(v)) => {
        let mut deduplicate = FastHashMap::<NodeIdentity, ValueChange<V>>::default();

        v.into_iter()
          .filter_map(|(key, delta)| {
            let delta = match delta {
              ValueChange::Delta(d, pd) => {
                let d = (self.downcast_delta)(d);
                let pd = pd.and_then(|pd| (self.downcast_delta)(pd));

                d.map(|d| ValueChange::Delta(d, pd))
              }
              ValueChange::Remove(d) => {
                (self.downcast_delta)(d).map(|mat| ValueChange::Remove(mat))
              }
            };
            delta.map(|v| ((s_id, key), v))
          })
          .for_each(|(key, d)| {
            if let Some(current) = deduplicate.get_mut(&key) {
              if !current.merge(&d) {
                deduplicate.remove(&key);
              }
            } else {
              deduplicate.insert(key, d);
            }
          });

        Poll::Ready(Box::new(deduplicate))
      }

      _ => Poll::Pending,
    }
  }

  fn extra_request(&mut self, _: &mut ExtraCollectionOperation) {}

  fn access(&self) -> PollCollectionCurrent<NodeIdentity, V> {
    let tree = Arc::new(self.inner.derived_tree.read());
    let access = move |(s_id, idx): &NodeIdentity| {
      if *s_id == self.scene_id {
        let handle = tree.try_recreate_handle(*idx)?;
        tree
          .try_get_node(handle)
          .map(|node| (self.getter)(&node.data().data))
      } else {
        None
      }
    };
    let access_c = access.clone();

    // todo, avoid clone by unsafe
    let tree = self.inner.derived_tree.read();

    let make_iter = move || {
      let access_c = access_c.clone();
      let iter = tree
        .iter_node_idx()
        .map(move |v| (self.scene_id, v))
        .collect::<Vec<_>>()
        .into_iter()
        .filter_map(move |k| access_c(&k).map(|v| (k, v)));
      Box::new(iter) as Box<dyn Iterator<Item = (NodeIdentity, V)>>
    };

    let c = GeneralVirtualCollection {
      access: Arc::new(access),
      make_iter: Arc::new(make_iter),
    };

    // Box::new(c)
    todo!()
  }
}
