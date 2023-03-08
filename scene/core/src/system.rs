use crate::*;
use rendiation_geometry::Box3;

use futures::stream::*;
use futures::Stream;
use reactive::*;

#[allow(unused)]
pub struct SceneBoundingSystem {
  /// actually data
  models_bounding: Vec<Option<Box3>>,
  handler: SceneModelStream,
}

pub type BoxUpdate = VecUpdateUnit<Option<Box3>>;
type SceneModelStream = impl Stream<Item = BoxUpdate> + Unpin;

impl SceneBoundingSystem {
  pub fn new(scene: &Scene) -> Self {
    type BoxStream = impl Stream<Item = Option<Box3>> + Unpin;

    pub fn build_world_box_stream(model: &SceneModel) -> BoxStream {
      let world_mat_stream = model
        .listen_by(with_field!(SceneModelImpl => node))
        .map(|node| node.listen_by(with_field!(SceneNodeDataImpl => world_matrix)))
        .flatten_signal();

      let local_box_stream = model
        .listen_by(with_field!(SceneModelImpl => model))
        .map(|model| match model {
          SceneModelType::Standard(model) => Box::new(
            model
              .listen_by(with_field!(StandardModel => mesh))
              .map(|mesh| mesh.compute_local_bound()),
          ),
          SceneModelType::Foreign(_) => {
            Box::new(once_forever_pending(None)) as Box<dyn Unpin + Stream<Item = Option<Box3>>>
          }
        })
        .flatten_signal();

      local_box_stream
        .zip(world_mat_stream)
        .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)))
    }

    use arena::ArenaDelta::*;
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
      .map(|model_delta| match model_delta {
        Mutate((new, handle)) => (handle.index(), Some(build_world_box_stream(&new))),
        Insert((new, handle)) => (handle.index(), Some(build_world_box_stream(&new))),
        Remove(handle) => (handle.index(), None),
      })
      .flatten_into_vec_stream_signal();

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
