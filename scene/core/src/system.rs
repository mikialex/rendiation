use std::{
  ops::Deref,
  pin::Pin,
  task::{Context, Poll},
};

use crate::*;
use reactive::Stream;
use rendiation_geometry::Box3;

pub struct Pair<'a, T: IncrementalBase> {
  source: &'a T,
  delta: &'a Stream<T::Delta>,
}

pub enum Partial<'a, T: IncrementalBase> {
  All(&'a T),
  Delta(&'a T::Delta),
}

impl<'a, T: IncrementalBase> Pair<'a, T> {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(Partial<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    let sender_c = sender.clone();
    let send = move |mapped| {
      sender_c.unbounded_send(mapped).ok();
    };
    mapper(Partial::All(&self.source), &send);

    self.delta.on(move |v| {
      mapper(Partial::Delta(v), &send);
      sender.is_closed()
    });
    receiver
  }
}

impl<T: IncrementalBase> SceneItemRef<T> {
  pub fn pair(&self) -> Pair<T> {
    let inner = self.read();
    Pair {
      source: inner.deref(),
      delta: todo!(),
    }
  }
}

use futures::{
  future::{Pending, Ready},
  *,
};
type BoxStream = impl futures::Stream<Item = Option<Box3>>;
pub fn build_world_box_stream(model: &SceneModel) -> BoxStream {
  let world_mat_stream = model
    .pair()
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
        let node_d: Pair<SceneNodeDataImpl> = todo!();
        node_d.listen_by(|view, send| match view {
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
    .pair()
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
            .pair()
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

#[derive(Default)]
struct SceneBoxUpdater {
  changed: Vec<usize>,
  sub_streams: Vec<BoxStream>,

  /// actually data
  models_bounding: Vec<Option<Box3>>,
}

impl Clone for SceneBoxUpdater {
  fn clone(&self) -> Self {
    todo!()
  }
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
    todo!()
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
    let waker = todo!(); // change_notifier;
    let cx = Context::from_waker(waker);

    while let Poll::Ready(Some(update)) = self.poll_next_unpin(&mut cx) {
      // collect box updates
      // send into downstream stream TODO
      // update cache,
      println!("{update:?}");
      match update {
        BoxUpdate::Remove(_) => {}
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
      .pair()
      .listen_by(|view, send| match view {
        // simply trigger all model add deltas
        // but without trigger all unnecessary other scene deltas
        Partial::All(scene) => scene.models.expand(send),
        Partial::Delta(delta) => match delta {
          SceneInnerDelta::models(model_delta) => send(model_delta.clone()),
          _ => {}
        },
      })
      .map(move |model_delta| {
        let mut scene_updater = updater.clone();
        match model_delta {
          arena::ArenaDelta::Mutate((new, handle)) => {
            //
          }
          arena::ArenaDelta::Insert((new, handle)) => {
            // let box_stream = build_world_box_stream(new);
          }
          arena::ArenaDelta::Remove(handle) => {}
        }
      });

    // let reactive: Arc<RwLock<Vec<Option<MeshBoxReactiveCache>>>> = Default::default();
    // let weak_reactive = Arc::downgrade(&reactive);

    // let model_stream: Stream<arena::ArenaDelta<SceneModel>> = scene
    //   .read()
    //   .delta_stream
    //   .filter_map_ref(move |view| match view.delta {
    //     SceneInnerDelta::models(model_delta) => Some(model_delta),
    //     _ => None,
    //   });

    // let bounding_change_stream: Stream<BoxUpdate> = Default::default();
    // let box_c = bounding_change_stream.clone();

    // model_stream.on(move |model_delta| {
    //   if let Some(reactive) = weak_reactive.upgrade() {
    //     let mut reactive = reactive.write().unwrap();

    //     match model_delta {
    //       arena::ArenaDelta::Mutate((new_model, handle)) => {
    //         reactive[handle.into_raw_parts().0] =
    //           MeshBoxReactiveCache::from_model(new_model, *handle, &box_c);
    //       }
    //       arena::ArenaDelta::Insert((model, handle)) => {
    //         box_c.emit(&BoxUpdate::Active(*handle));

    //         let r = MeshBoxReactiveCache::from_model(model, *handle, &box_c);
    //         if handle.into_raw_parts().0 == reactive.len() {
    //           reactive.push(r);
    //         } else {
    //           reactive[handle.into_raw_parts().0] = r;
    //         }
    //       }
    //       arena::ArenaDelta::Remove(handle) => {
    //         reactive[handle.into_raw_parts().0] = None;
    //         box_c.emit(&BoxUpdate::Remove(*handle));
    //       }
    //     }

    //     false
    //   } else {
    //     true
    //   }
    // });

    // let update_queue: Arc<RwLock<Vec<BoxUpdate>>> = Default::default();
    // let update_queue_weak = Arc::downgrade(&update_queue);
    // bounding_change_stream.on(move |delta| {
    //   if let Some(update_queue) = update_queue_weak.upgrade() {
    //     update_queue.write().unwrap().push(delta.clone());
    //     false
    //   } else {
    //     true
    //   }
    // });

    Self {
      handler,
      models_bounding: Default::default(),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.into_raw_parts().0]
  }

  // pub fn get_bounding_change_stream(&self) -> &Stream<BoxUpdate> {
  //   &self.bounding_change_stream
  // }
}
