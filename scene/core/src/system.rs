use std::{
  pin::Pin,
  task::{Context, Poll, Waker},
};

use crate::*;
use rendiation_geometry::Box3;

use futures::stream::*;
use futures::Stream;
use std::future::ready;

type BoxStream = impl Stream<Item = Option<Box3>> + Unpin;
pub fn build_world_box_stream(model: &SceneModel) -> BoxStream {
  let world_mat_stream = model
    .listen_by(with_field!(SceneModelImpl => node))
    .map(|node| node.visit(|node| node.listen_by(with_field!(SceneNodeDataImpl => world_matrix))))
    .flatten();

  let local_box_stream = model
    .listen_by(with_field!(SceneModelImpl => model))
    .map(|model| match model {
      SceneModelType::Standard(model) => Some(model),
      SceneModelType::Foreign(_) => None,
    })
    .map(|model| {
      if let Some(model) = model {
        Box::new(
          model
            .listen_by(with_field!(StandardModel => mesh))
            .map(|mesh| mesh.compute_local_bound()),
        )
      } else {
        Box::new(once(ready(None)).chain(pending())) as Box<dyn Unpin + Stream<Item = Option<Box3>>>
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
  changed: Arc<RwLock<Vec<SceneModelHandle>>>,
  sub_streams: Vec<Option<BoxStream>>,
  waker: Option<Waker>,
}

struct ChangeWaker {
  index: SceneModelHandle,
  changed: Arc<RwLock<Vec<SceneModelHandle>>>,
  waker: Waker,
}

impl futures::task::ArcWake for ChangeWaker {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self.changed.write().unwrap().push(arc_self.index);
    arc_self.waker.wake_by_ref();
  }
}

impl Stream for SceneBoxUpdater {
  type Item = BoxUpdate;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut inner = self.inner.write().unwrap();
    let inner: &mut SceneBoxUpdaterInner = &mut inner;
    let mut changed = inner.changed.write().unwrap(); // todo deadlock
    if let Some(index) = changed.pop() {
      let waker = inner.waker.get_or_insert_with(|| cx.waker().clone());
      let waker = Arc::new(ChangeWaker {
        waker: waker.clone(),
        index,
        changed: inner.changed.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx = Context::from_waker(&waker);

      if let Some(stream) = inner.sub_streams.get_mut(index.index()).unwrap() {
        stream
          .poll_next_unpin(&mut cx)
          .map(|r| r.map(|r| BoxUpdate::Update { index, bbox: r }))
      } else {
        Poll::Pending
      }
    } else {
      Poll::Pending
    }
  }
}

type SceneModelStream = impl Stream<Item = BoxUpdate> + Unpin;

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

impl SceneBoundingSystem {
  pub fn maintain(&mut self) {
    do_updates(&mut self.handler, |update| {
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
    })
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
              scene_updater.changed.write().unwrap().push(handle);
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

pub fn do_updates<T: Stream + Unpin>(stream: &mut T, mut on_update: impl FnMut(T::Item)) {
  // synchronously polling the stream, pull all box update.
  // note, if the compute stream contains async mapping, the async part is actually
  // polled inactively.
  let waker = futures::task::noop_waker_ref();
  let mut cx = Context::from_waker(waker);
  while let Poll::Ready(Some(update)) = stream.poll_next_unpin(&mut cx) {
    on_update(update)
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
