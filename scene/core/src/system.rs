use std::{
  ops::Deref,
  pin::Pin,
  sync::Weak,
  task::{Context, Poll},
};

use crate::*;
use reactive::Stream;
use rendiation_geometry::Box3;

pub trait Signal {
  type Item;

  fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<SignalState<Self::Item>>;
}

pub enum SignalState<T> {
  Changed(T),
  Terminated,
}

pub trait Value {
  type Item;
  fn get(&self) -> &Self::Item;
}

struct Reactive<T> {
  inner: Arc<RwLock<T>>,
}

struct ReactiveSignal<T> {
  inner: Weak<RwLock<T>>,
  changed: bool,
}

// impl<T> Signal for ReactiveSignal<T> {
//   type Item = T;

//   fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<SignalState<Self::Item>> {
//     if self.changed {
//       if let Some(inner) = self.inner.upgrade() {
//         Poll::Ready(inner.clone())
//       } else {
//         Poll::Ready(SignalState::Terminated)
//       }
//     } else {
//       Poll::Pending
//     }
//   }
// }

trait DeltaReducer<T: IncrementalBase> {
  type Target;
  fn create_init(&self, value: &T) -> Self::Target;
  fn map_delta(&self, delta: &T::Delta) -> Self::Target;
}

// type ModelWorldBox = impl futures::Stream<Item =Option<Box3>>;

pub struct Pair<'a, T: IncrementalBase> {
  source: &'a T,
  delta: &'a Stream<T::Delta>,
}

pub enum EntireOrDeltaRef<'a, T: IncrementalBase> {
  Entire(&'a T),
  Delta(&'a T::Delta),
}

impl<'a, T: IncrementalBase> Pair<'a, T> {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(EntireOrDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    let sender_c = sender.clone();
    let send = move |mapped| {
      sender_c.unbounded_send(mapped);
    };
    mapper(EntireOrDeltaRef::Entire(&self.source), &send);

    self.delta.on(move |v| {
      mapper(EntireOrDeltaRef::Delta(v), &send);
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

use futures::*;
pub fn build_world_box_stream(model: &SceneModel) -> impl futures::Stream<Item = Option<Box3>> {
  let world_mat_stream = model
    .pair()
    .listen_by(|view, send| match view {
      EntireOrDeltaRef::Entire(model) => send(model.node.clone()),
      EntireOrDeltaRef::Delta(delta) => match delta {
        SceneModelImplDelta::node(node) => send(node.clone()),
        _ => {}
      },
    })
    .map(|node| {
      node.visit(|node| {
        let node_d: Pair<SceneNodeDataImpl> = todo!();
        node_d.listen_by(|view, send| match view {
          EntireOrDeltaRef::Entire(node) => send(node.world_matrix()),
          EntireOrDeltaRef::Delta(d) => match d {
            SceneNodeDataImplDelta::world_matrix(mat) => send(*mat),
            _ => {}
          },
        })
      })
    })
    .flatten();

  // impl futures::Stream<Item = Option<Box3>>
  let local_box_stream = model
    .pair()
    .listen_by(|view, send| match view {
      EntireOrDeltaRef::Entire(model) => send(model.model.clone()),
      EntireOrDeltaRef::Delta(delta) => match delta {
        SceneModelImplDelta::model(model) => send(model.clone()),
        _ => {}
      },
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
              EntireOrDeltaRef::Entire(model) => send(model.mesh.clone()),
              EntireOrDeltaRef::Delta(delta) => match delta {
                StandardModelDelta::mesh(mesh) => send(mesh.clone()),
                _ => {}
              },
            })
            .map(|mesh| mesh.read().compute_local_bound()),
        )
      } else {
        Box::new(todo!())
      }
    })
    .flatten();

  local_box_stream
    .zip(world_mat_stream)
    .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)))
}

#[allow(unused)]
pub struct SceneBoundingSystem {
  model_delta: Stream<arena::ArenaDelta<SceneModel>>,

  /// actually data
  models_bounding: Vec<Option<Box3>>,

  reactive: Arc<RwLock<Vec<Option<MeshBoxReactiveCache>>>>,

  update_queue: Arc<RwLock<Vec<BoxUpdate>>>,

  /// for outside user subscribe
  bounding_change_stream: Stream<BoxUpdate>,
}

#[allow(unused)]
struct MeshBoxReactiveCache {
  model_node_stream: Stream<SceneNode>,
  local_box_stream: Stream<Option<Box3>>,
  world_mat_stream: Stream<Mat4<f32>>,
  mesh_stream: Stream<SceneMesh>,
  world_box_stream: Stream<Option<Box3>>,
}

impl MeshBoxReactiveCache {
  pub fn from_model(
    model: &SceneModel,
    model_handle: SceneModelHandle,
    out_stream: &Stream<BoxUpdate>,
  ) -> Option<Self> {
    let model = model.read();
    let model_node_stream = model.delta_stream.filter_map_ref(|node| match node.delta {
      SceneModelImplDelta::node(node) => Some(node),
      _ => None,
    });
    let world_mat_stream = model_node_stream
      .map(|node| {
        node.visit(|node| {
          node.delta_stream.filter_map_ref(|d| match d.delta {
            SceneNodeDataImplDelta::world_matrix(mat) => Some(mat),
            _ => None,
          })
        })
      })
      .flatten();
    match &model.model {
      SceneModelType::Standard(model) => {
        let mesh_stream = model
          .read()
          .delta_stream
          .filter_map_ref(move |view| match view.delta {
            StandardModelDelta::mesh(mesh_delta) => Some(mesh_delta),
            _ => None,
          });

        // todo: we not handle mesh internal change to box change, just recompute box when mesh reference changed
        let local_box_stream = mesh_stream.map(|mesh| mesh.read().compute_local_bound());

        let world_box_stream = local_box_stream
          .merge_map(&world_mat_stream, |local_box, world_mat| {
            local_box.map(|b| b.apply_matrix_into(*world_mat))
          });

        let r = MeshBoxReactiveCache {
          model_node_stream,
          local_box_stream,
          world_mat_stream,
          mesh_stream,
          world_box_stream,
        };

        let out_stream = out_stream.clone();

        r.world_box_stream.on(move |b| {
          out_stream.emit(&BoxUpdate::Update {
            index: model_handle,
            bbox: *b,
          });
          false
        });
        r.into()
      }
      SceneModelType::Foreign(_) => None,
    }
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

impl SceneBoundingSystem {
  pub fn maintain(&mut self) {
    self
      .update_queue
      .write()
      .unwrap()
      .drain(..)
      .for_each(|update| {
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
      })
  }

  pub fn new(scene: &Scene) -> Self {
    let reactive: Arc<RwLock<Vec<Option<MeshBoxReactiveCache>>>> = Default::default();
    let weak_reactive = Arc::downgrade(&reactive);

    let model_stream: Stream<arena::ArenaDelta<SceneModel>> = scene
      .read()
      .delta_stream
      .filter_map_ref(move |view| match view.delta {
        SceneInnerDelta::models(model_delta) => Some(model_delta),
        _ => None,
      });

    let bounding_change_stream: Stream<BoxUpdate> = Default::default();
    let box_c = bounding_change_stream.clone();

    model_stream.on(move |model_delta| {
      if let Some(reactive) = weak_reactive.upgrade() {
        let mut reactive = reactive.write().unwrap();

        match model_delta {
          arena::ArenaDelta::Mutate((new_model, handle)) => {
            reactive[handle.into_raw_parts().0] =
              MeshBoxReactiveCache::from_model(new_model, *handle, &box_c);
          }
          arena::ArenaDelta::Insert((model, handle)) => {
            box_c.emit(&BoxUpdate::Active(*handle));

            let r = MeshBoxReactiveCache::from_model(model, *handle, &box_c);
            if handle.into_raw_parts().0 == reactive.len() {
              reactive.push(r);
            } else {
              reactive[handle.into_raw_parts().0] = r;
            }
          }
          arena::ArenaDelta::Remove(handle) => {
            reactive[handle.into_raw_parts().0] = None;
            box_c.emit(&BoxUpdate::Remove(*handle));
          }
        }

        false
      } else {
        true
      }
    });

    let update_queue: Arc<RwLock<Vec<BoxUpdate>>> = Default::default();
    let update_queue_weak = Arc::downgrade(&update_queue);
    bounding_change_stream.on(move |delta| {
      if let Some(update_queue) = update_queue_weak.upgrade() {
        update_queue.write().unwrap().push(delta.clone());
        false
      } else {
        true
      }
    });

    Self {
      model_delta: model_stream,
      models_bounding: Default::default(),
      reactive,
      update_queue,
      bounding_change_stream,
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Option<Box3> {
    &self.models_bounding[handle.into_raw_parts().0]
  }

  pub fn get_bounding_change_stream(&self) -> &Stream<BoxUpdate> {
    &self.bounding_change_stream
  }
}
