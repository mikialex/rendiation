use std::{
  pin::Pin,
  task::{Context, Poll},
};

use crate::*;
use rendiation_geometry::Box3;

use futures::*;
type BoxStream = impl futures::Stream<Item = Option<Box3>>;
pub fn build_world_box_stream(model: &SceneModel) -> BoxStream {
  let world_mat_stream = model
    .listen_by(|view, send| match view {
      Partial::All(model) => send(model.node.clone()),
      Partial::Delta(delta) => {
        if let SceneModelImplDelta::node(node) = delta {
          send(node.clone())
        }
      }
    })
    .map(|node| {
      node.visit(|node| {
        node.listen_by(|view, send| match view {
          Partial::All(node) => send(node.world_matrix()),
          Partial::Delta(d) => {
            if let SceneNodeDataImplDelta::world_matrix(mat) = d {
              send(*mat)
            }
          }
        })
      })
    })
    .flatten();

  let local_box_stream = model
    .listen_by(|view, send| match view {
      Partial::All(model) => send(model.model.clone()),
      Partial::Delta(delta) => {
        if let SceneModelImplDelta::model(model) = delta {
          send(model.clone())
        }
      }
    })
    .map(|model| match model {
      SceneModelType::Standard(model) => Some(model),
      SceneModelType::Foreign(_) => None,
    })
    .map(|model| {
      if let Some(model) = model {
        Box::new(
          model
            .listen_by(|view, send| match view {
              Partial::All(model) => send(model.mesh.clone()),
              Partial::Delta(delta) => {
                if let StandardModelDelta::mesh(mesh) = delta {
                  send(mesh.clone())
                }
              }
            })
            .map(|mesh| mesh.read().compute_local_bound()),
        )
      } else {
        Box::new(futures::stream::once(std::future::ready(None)).chain(futures::stream::pending()))
          as Box<dyn Unpin + futures::Stream<Item = Option<Box3>>>
      }
    })
    .flatten();

  local_box_stream
    .zip(world_mat_stream)
    .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)))
}

#[derive(Default, Clone)]
struct SceneBoxUpdater {
  inner: Arc<RwLock<SceneBoxUpdaterInner>>,
}

#[derive(Default)]
struct SceneBoxUpdaterInner {
  changed: Vec<usize>,
  sub_streams: Vec<Option<BoxStream>>,
}

impl futures::Stream for SceneBoxUpdater {
  type Item = (SceneModelHandle, Option<Box3>);

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    todo!()
  }
}

type SceneModelStream = impl futures::Stream<Item = ()>;

#[allow(unused)]
pub struct SceneBoundingSystem {
  /// actually data
  models_bounding: Vec<Option<Box3>>,

  handler: SceneModelStream,
}

impl futures::Stream for SceneBoundingSystem {
  type Item = BoxUpdate;

  // do updating, update could be batched
  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    // self.handler.p
  }
}

#[derive(Clone, Debug)]
pub enum BoxUpdate {
  Remove(SceneModelHandle),
  Active(SceneModelHandle),
  Update {
    index: SceneModelHandle,
    bbox: Option<Box3>,
  },
}

impl Unpin for SceneBoundingSystem {}

impl SceneBoundingSystem {
  pub fn maintain(&mut self) {
    // synchronously polling the stream, pull all box update.
    // note, if the compute stream contains async mapping, the async part is actually
    // polled inactively.
    let waker = futures::task::noop_waker_ref();
    let mut cx = Context::from_waker(waker);

    while let Poll::Ready(Some(update)) = self.poll_next_unpin(&mut cx) {
      // collect box updates
      // send into downstream stream TODO
      // update cache,
      println!("{update:?}");
      match update {
        BoxUpdate::Remove(index) => {
          self.models_bounding[index.into_raw_parts().0] = None;
        }
        BoxUpdate::Active(index) => {
          if index.into_raw_parts().0 == self.models_bounding.len() {
            self.models_bounding.push(None);
          }
        }
        BoxUpdate::Update { index, bbox } => {
          self.models_bounding[index.into_raw_parts().0] = bbox;
        }
      }
    }
  }

  pub fn new(scene: &Scene) -> Self {
    let updater = SceneBoxUpdater::default();

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
      .map(move |model_delta| {
        let scene_updater = updater.clone();
        let mut scene_updater = scene_updater.inner.write().unwrap();
        match model_delta {
          arena::ArenaDelta::Mutate((new, handle)) => {
            scene_updater.sub_streams[handle.into_raw_parts().0] =
              Some(build_world_box_stream(&new));
          }
          arena::ArenaDelta::Insert((new, handle)) => {
            let handler = Some(build_world_box_stream(&new));
            if handle.into_raw_parts().0 == scene_updater.sub_streams.len() {
              scene_updater.sub_streams.push(handler);
            } else {
              scene_updater.sub_streams[handle.into_raw_parts().0] = handler;
            }
          }
          arena::ArenaDelta::Remove(handle) => {
            scene_updater.sub_streams[handle.into_raw_parts().0] = None;
          }
        }
      });

    Self {
      handler,
      models_bounding: Default::default(),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.into_raw_parts().0]
  }
}
