use crate::*;
use futures::Stream;
use reactive::*;
use tree::CoreTree;
use tree::ParentTree;
use tree::ParentTreeDirty;
use tree::TreeHierarchyDerivedSystem;

#[derive(Clone)]
pub struct SceneNodeDeriveSystem {
  inner: Arc<RwLock<SceneNodeDeriveSystemInner>>,
}

struct SceneNodeDeriveSystemInner {
  inner:
    TreeHierarchyDerivedSystem<SceneNodeDerivedData, ParentTreeDirty<SceneNodeDeriveDataDirtyFlag>>,
  updater: StreamCacheUpdate,
}
type SingleSceneNodeChangeStream = impl Stream<Item = SceneNodeDerivedDataDelta> + Unpin;
type SceneNodeChangeStream = impl Stream<Item = (usize, SceneNodeDerivedDataDelta)> + Unpin;

pub type SceneNodeChangeStreamIndexMapper =
  StreamBroadcaster<SceneNodeChangeStream, SceneNodeDerivedDataDelta, IndexMapping>;

pub type SingleSceneNodeChangeStreamFanOut =
  StreamBroadcaster<SingleSceneNodeChangeStream, SceneNodeDerivedDataDelta, FanOut>;

type StreamCacheUpdate = impl Stream + Unpin + AsRef<StreamVec<SingleSceneNodeChangeStreamFanOut>>;

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
      // we don't care about deletions in this stream
      .filter_map_sync(|d: (usize, Option<SceneNodeDerivedDataDelta>)| d.1.map(|d1| (d.0, d1)))
      .create_index_mapping_broadcaster();

    let sub_broad_caster = StreamVec::<SingleSceneNodeChangeStreamFanOut>::default();

    let stream_cache_updating = inner_sys.derived_stream.fork_stream().fold_signal_flatten(
      sub_broad_caster,
      move |(idx, delta), sub_broad_caster| {
        if delta.is_none() {
          sub_broad_caster.insert(idx, None)
          // we check if is none first to avoid too much sub stream recreate
        } else if sub_broad_caster.get(idx).is_none() {
          sub_broad_caster.insert(
            idx,
            Some(
              indexed_stream_mapper
                .create_sub_stream_by_index(idx)
                .create_broad_caster(),
            ),
          )
        }
        None
      },
    );

    let inner = SceneNodeDeriveSystemInner {
      inner: inner_sys,
      updater: stream_cache_updating,
    };
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  pub fn maintain(&mut self) {
    let mut inner = self.inner.write().unwrap();
    do_updates(&mut inner.updater, |_| {});
  }
}

pub type WorldMatrixStream = impl Stream<Item = Mat4<f32>>;

impl SceneNodeDeriveSystem {
  pub fn get_world_matrix(&self, node: &SceneNode) -> Mat4<f32> {
    self.get_world_matrix_by_raw_handle(node.raw_handle().index())
  }

  pub fn get_world_matrix_by_raw_handle(&self, index: usize) -> Mat4<f32> {
    self.inner.read().unwrap().inner.visit_derived_tree(|tree| {
      let handle = tree.recreate_handle(index);
      tree.get_node(handle).data().data.world_matrix
    })
  }
  pub fn visit_derived<R>(&self, index: usize, v: impl FnOnce(&SceneNodeDerivedData) -> R) -> R {
    self.inner.read().unwrap().inner.visit_derived_tree(|tree| {
      let handle = tree.recreate_handle(index);
      v(&tree.get_node(handle).data().data)
    })
  }

  pub fn create_derived_stream_by_raw_handle(
    &self,
    index: usize,
  ) -> impl Stream<Item = SceneNodeDerivedDataDelta> {
    let derived = self.visit_derived(index, |d| d.clone());
    let init_deltas = expand_out(&derived);
    self
      .inner
      .read()
      .unwrap()
      .updater
      .as_ref()
      .get(index)
      .unwrap()
      .fork_stream_with_init(init_deltas)
  }

  pub fn create_world_matrix_stream(&self, node: &SceneNode) -> WorldMatrixStream {
    self.create_world_matrix_stream_by_raw_handle(node.raw_handle().index())
  }
  pub fn create_world_matrix_stream_by_raw_handle(&self, index: usize) -> WorldMatrixStream {
    self
      .create_derived_stream_by_raw_handle(index)
      .filter_map_sync(|d| match d {
        SceneNodeDerivedDataDelta::world_matrix(m) => Some(m),
        SceneNodeDerivedDataDelta::net_visible(_) => None,
      })
  }
  pub fn get_net_visible(&self, node: &SceneNode) -> bool {
    self.inner.read().unwrap().inner.visit_derived_tree(|tree| {
      let handle = tree.recreate_handle(node.raw_handle().index());
      tree.get_node(handle).data().data.net_visible
    })
  }
}
