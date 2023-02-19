use std::{
  pin::Pin,
  task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use crate::*;
use rendiation_geometry::Box3;

use futures::*;
type BoxStream = impl futures::Stream<Item = Option<Box3>> + Unpin;
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
            .map(|mesh| mesh.compute_local_bound()),
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
  type Item = BoxUpdate;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut inner = self.inner.write().unwrap();
    // let changed = unsafe { inner.get_unchecked_mut().changed };
    if let Some(index) = inner.changed.pop() {
      let vtable = RawWakerVTable::new();
      let raw_waker = RawWaker::new(todo!(), &vtable);
      let waker = unsafe { Waker::from_raw(raw_waker) };
      let mut cx = Context::from_waker(&waker);

      if let Some(stream) = inner.sub_streams.get_mut(index).unwrap() {
        stream.poll_next_unpin(&mut cx);
      }

      todo!()
    } else {
      Poll::Pending
    }
  }
}

type SceneModelStream = impl futures::Stream<Item = BoxUpdate> + Unpin;

#[allow(unused)]
pub struct SceneBoundingSystem {
  /// actually data
  models_bounding: Vec<Option<Box3>>,
  updater: SceneBoxUpdater,
  handler: SceneModelStream,
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

    while let Poll::Ready(Some(update)) = self.handler.poll_next_unpin(&mut cx) {
      // collect box updates
      // send into downstream stream TODO
      // update cache,
      println!("{update:?}");
      match update {
        BoxUpdate::Remove(index) => {
          self.models_bounding[index.index()] = None;
        }
        BoxUpdate::Active(index) => {
          if index.index() == self.models_bounding.len() {
            self.models_bounding.push(None);
          }
        }
        BoxUpdate::Update { index, bbox } => {
          self.models_bounding[index.index()] = bbox;
        }
      }
    }
  }

  pub fn new(scene: &Scene) -> Self {
    let updater = SceneBoxUpdater::default();
    let updater_c = updater.clone();

    let scene_model_handler = scene
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
      .filter_map(move |model_delta| {
        let scene_updater = updater_c.clone();
        async move {
          let mut scene_updater = scene_updater.inner.write().unwrap();
          match model_delta {
            arena::ArenaDelta::Mutate((new, handle)) => {
              scene_updater.sub_streams[handle.index()] = Some(build_world_box_stream(&new));
              scene_updater.changed.push(handle.index());
              None
            }
            arena::ArenaDelta::Insert((new, handle)) => {
              let handler = Some(build_world_box_stream(&new));
              if handle.index() == scene_updater.sub_streams.len() {
                scene_updater.sub_streams.push(handler);
              } else {
                scene_updater.sub_streams[handle.index()] = handler;
              }
              Some(BoxUpdate::Active(handle))
            }
            arena::ArenaDelta::Remove(handle) => {
              scene_updater.sub_streams[handle.index()] = None;
              Some(BoxUpdate::Remove(handle))
            }
          }
        }
      });

    let handler = Box::pin(futures::stream::select(
      scene_model_handler,
      updater.clone(),
    ));

    Self {
      handler,
      updater,
      models_bounding: Default::default(),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.index()]
  }
}

// trait EarlyTerminateStreamExt: Stream {
//   fn flatten_outside(self) -> FlattenOutSide<Self>
//   where
//     Self::Item: Stream,
//     Self: Sized;
// }

// pin_project! {
//     /// Stream for the [`flatten`](super::StreamExt::flatten) method.
//     #[derive(Debug)]
//     #[must_use = "streams do nothing unless polled"]
//     pub struct FlattenOutSide<St, U> {
//         #[pin]
//         stream: St,
//         #[pin]
//         next: Option<U>,
//     }
// }

// impl<St, U> FlattenOutSide<St, U> {
//   pub(super) fn new(stream: St) -> Self {
//     Self { stream, next: None }
//   }

//   delegate_access_inner!(stream, St, ());
// }

// impl<St> FusedStream for FlattenOutSide<St, St::Item>
// where
//   St: FusedStream,
//   St::Item: Stream,
// {
//   fn is_terminated(&self) -> bool {
//     self.next.is_none() && self.stream.is_terminated()
//   }
// }

// impl<St> Stream for FlattenOutSide<St, St::Item>
// where
//   St: Stream,
//   St::Item: Stream,
// {
//   type Item = <St::Item as Stream>::Item;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//     let mut this = self.project();
//     Poll::Ready(loop {
//       if let Some(s) = this.next.as_mut().as_pin_mut() {
//         if let Some(item) = ready!(s.poll_next(cx)) {
//           break Some(item);
//         } else {
//           this.next.set(None);
//         }
//       } else if let Some(s) = ready!(this.stream.as_mut().poll_next(cx)) {
//         this.next.set(Some(s));
//       } else {
//         break None;
//       }
//     })
//   }
// }
