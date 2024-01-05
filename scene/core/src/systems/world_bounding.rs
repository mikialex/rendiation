use futures::stream::*;
use reactive::*;
use rendiation_geometry::Box3;

use crate::*;

#[pin_project::pin_project]
pub struct SceneModelWorldBoundingSystem {
  /// actually data
  models_bounding: Vec<Option<Box3>>,
  handler: StreamForker<SceneModelStream>,
}

pub type BoxUpdate = VecUpdateUnit<Option<Box3>>;

type SceneModelStream = impl Stream<Item = BoxUpdate> + Unpin;

impl SceneModelWorldBoundingSystem {
  pub fn new(scene: &SceneCore, d_sys: &SceneNodeDeriveSystem) -> Self {
    fn build_world_box_stream(
      model: &SceneModel,
      d_sys: &SceneNodeDeriveSystem,
    ) -> impl Stream<Item = Option<Box3>> + Unpin + 'static {
      let d_sys = d_sys.clone();
      let world_mat_stream = model
        .unbound_listen_by(with_field!(SceneModelImpl => node))
        .filter_map_sync(move |node| d_sys.create_world_matrix_stream(&node))
        .flatten_signal();

      let local_box_stream = model
        .unbound_listen_by(with_field!(SceneModelImpl => model))
        .map(|model| match model {
          ModelEnum::Standard(model) => Box::new(
            model
              .unbound_listen_by(with_field!(StandardModel => mesh))
              .map(|mesh| mesh.build_local_bound_stream())
              .flatten_signal(),
          ),
          ModelEnum::Foreign(_) => {
            Box::new(once_forever_pending(None)) as Box<dyn Unpin + Stream<Item = Option<Box3>>>
          }
        })
        .flatten_signal();

      local_box_stream
        .zip_signal(world_mat_stream)
        .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)))
    }

    use arena::ArenaDelta::*;
    let d_sys = d_sys.clone();
    let handler = scene
      .unbound_listen_by(with_field_expand!(SceneCoreImpl => models))
      .map(move |model_delta| match model_delta {
        Mutate((new, handle)) => (handle.index(), Some(build_world_box_stream(&new, &d_sys))),
        Insert((new, handle)) => (handle.index(), Some(build_world_box_stream(&new, &d_sys))),
        Remove(handle) => (handle.index(), None),
      })
      .flatten_into_vec_stream_signal()
      .create_broad_caster();

    Self {
      handler,
      models_bounding: Default::default(),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.index()]
  }
}

impl Stream for SceneModelWorldBoundingSystem {
  type Item = ();

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    let this = self.project();
    if this.handler.poll_until_pending_or_terminate(cx, |update| {
      // collect box updates
      // send into downstream stream TODO
      // update cache,
      match update {
        BoxUpdate::Remove(index) => {
          this.models_bounding[index] = None;
        }
        BoxUpdate::Active(index) => {
          if index == this.models_bounding.len() {
            this.models_bounding.push(None);
          }
        }
        BoxUpdate::Updates(mut updates) => {
          for IndexedItem { index, item } in updates.drain(..) {
            this.models_bounding[index] = item;
          }
        }
      }
    }) {
      return std::task::Poll::Ready(None);
    }
    std::task::Poll::Pending
  }
}
