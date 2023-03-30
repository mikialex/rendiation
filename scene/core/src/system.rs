use crate::*;
use rendiation_geometry::Box3;

use futures::stream::*;
use futures::Stream;
use reactive::*;
use tree::CoreTree;
use tree::TreeHierarchyDerivedSystem;

#[derive(Clone)]
pub struct SceneNodeDeriveSystem {
  pub(crate) inner: TreeHierarchyDerivedSystem<SceneNodeDerivedData>,
  indexed_stream_mapper: SceneNodeChangeStreamIndexMapper,
}

type SceneNodeChangeStream = impl Stream<Item = (usize, SceneNodeDerivedDataDelta)> + Unpin;

type SceneNodeChangeStreamIndexMapper =
  StreamBoardCaster<SceneNodeChangeStream, SceneNodeDerivedDataDelta, IndexMapping>;

impl SceneNodeDeriveSystem {
  pub fn new(nodes: &SceneNodeCollection) -> Self {
    let mut expect = None;
    nodes.inner.visit_inner(|tree| {
      let stream = tree.source.listen();
      expect = TreeHierarchyDerivedSystem::<SceneNodeDerivedData>::new(stream, &nodes.inner).into();
    });
    let inner_sys = expect.unwrap();

    let indexed_stream_mapper: SceneNodeChangeStreamIndexMapper = inner_sys
      .derived_stream
      .fork_stream()
      .create_index_mapping_boardcaster();

    Self {
      inner: inner_sys,
      indexed_stream_mapper,
    }
  }
}

impl SceneNodeDeriveSystem {
  pub fn get_world_matrix(&self, node: &SceneNode) -> Mat4<f32> {
    self.inner.visit_derived_tree(|tree| {
      let handle = tree.recreate_handle(node.raw_handle().index());
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

pub struct SceneBoundingSystem {
  /// actually data
  models_bounding: Vec<Option<Box3>>,
  handler: StreamForker<SceneModelStream>,
}

pub type BoxUpdate = VecUpdateUnit<Option<Box3>>;

type SceneModelStream = impl Stream<Item = BoxUpdate> + Unpin;

impl SceneBoundingSystem {
  pub fn new(scene: &Scene, d_sys: &SceneNodeDeriveSystem) -> Self {
    type BoxStream = impl Stream<Item = Option<Box3>> + Unpin;

    fn build_world_box_stream(
      model: &SceneModel,
      filter: SceneNodeChangeStreamIndexMapper,
    ) -> BoxStream {
      let world_mat_stream = model
        .listen_by(with_field!(SceneModelImpl => node))
        .map(move |node| {
          filter
            .create_sub_stream_by_index(node.raw_handle().index())
            .filter_map_sync(|d| match d {
              SceneNodeDerivedDataDelta::world_matrix(m) => Some(m),
              SceneNodeDerivedDataDelta::net_visible(_) => None,
            })
        })
        .flatten_signal();

      let local_box_stream = model
        .listen_by(with_field!(SceneModelImpl => model))
        .map(|model| match model {
          SceneModelType::Standard(model) => Box::new(
            model
              .listen_by(with_field!(StandardModel => mesh))
              .map(|mesh| mesh.build_local_bound_stream())
              .flatten_signal(),
          ),
          SceneModelType::Foreign(_) => {
            Box::new(once_forever_pending(None)) as Box<dyn Unpin + Stream<Item = Option<Box3>>>
          }
        })
        .flatten_signal();

      local_box_stream
        .zip_signal(world_mat_stream)
        .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)))
    }

    use arena::ArenaDelta::*;
    let mapper = d_sys.indexed_stream_mapper.clone();
    let handler = scene
      .listen_by(|view, send| match view {
        // simply trigger all model add deltas
        // but not trigger all other unnecessary scene deltas
        Partial::All(scene) => scene.models.expand(send),
        Partial::Delta(delta) => {
          if let SceneInnerDelta::models(model_delta) = delta {
            send(model_delta.clone())
          }
        }
      })
      .map(move |model_delta| match model_delta {
        Mutate((new, handle)) => (
          handle.index(),
          Some(build_world_box_stream(&new, mapper.clone())),
        ),
        Insert((new, handle)) => (
          handle.index(),
          Some(build_world_box_stream(&new, mapper.clone())),
        ),
        Remove(handle) => (handle.index(), None),
      })
      .flatten_into_vec_stream_signal()
      .create_board_caster();

    Self {
      handler,
      models_bounding: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    do_updates(&mut self.handler, |update| {
      // collect box updates
      // send into downstream stream TODO
      // update cache,
      println!("{update:?}");
      match update {
        BoxUpdate::Remove(index) => {
          self.models_bounding[index] = None;
        }
        BoxUpdate::Active(index) => {
          if index == self.models_bounding.len() {
            self.models_bounding.push(None);
          }
        }
        BoxUpdate::Update { index, item } => {
          self.models_bounding[index] = item;
        }
      }
    })
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.index()]
  }
}
