use crate::*;
use reactive::Stream;
use rendiation_geometry::Box3;

// type ModelWorldBox = impl futures::Stream<Item =Option<Box3>>;

use futures::*;
pub fn build_world_box_stream(
  model: &SceneModel,
) -> Option<Box<dyn futures::Stream<Item = Option<Box3>>>> {
  let d: Stream<DeltaOf<SceneModelImpl>> = todo!();

  let world_mat_stream = d
    .listen()
    .filter_map(|node| async move {
      match node {
        SceneModelImplDelta::node(node) => Some(node.clone()),
        _ => None,
      }
    })
    .map(|node| {
      node.visit(|node| {
        let node_d: Stream<DeltaOf<SceneNodeDataImpl>> = todo!();
        node_d.listen().filter_map(|d| async move {
          match d {
            SceneNodeDataImplDelta::world_matrix(mat) => Some(mat),
            _ => None,
          }
        })
      })
    })
    .flatten();

  match &model.read().model {
    SceneModelType::Standard(model) => {
      let d: Stream<DeltaOf<StandardModel>> = todo!();
      let stream = d
        .listen()
        .filter_map(move |view| async move {
          match view {
            StandardModelDelta::mesh(mesh_delta) => Some(mesh_delta.clone()),
            _ => None,
          }
        })
        .map(|mesh| mesh.read().compute_local_bound())
        .zip(world_mat_stream)
        .map(|(local_box, world_mat)| local_box.map(|b| b.apply_matrix_into(world_mat)));

      Some(Box::new(stream))
    }
    SceneModelType::Foreign(_) => None,
  }
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
    out_stream: Stream<BoxUpdate>,
  ) -> Option<Self> {
    let model = model.read();
    let model_node_stream = model.delta_stream.filter_map(|node| match node.delta {
      SceneModelImplDelta::node(node) => Some(node.clone()),
      _ => None,
    });
    let world_mat_stream = model_node_stream
      .map(|node| {
        node.visit(|node| {
          node.delta_stream.filter_map(|d| match d.delta {
            SceneNodeDataImplDelta::world_matrix(mat) => Some(*mat),
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
          .filter_map(move |view| match view.delta {
            StandardModelDelta::mesh(mesh_delta) => Some(mesh_delta.clone()),
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

    let model_stream: Stream<arena::ArenaDelta<SceneModel>> =
      scene
        .read()
        .delta_stream
        .filter_map(move |view| match view.delta {
          SceneInnerDelta::models(model_delta) => Some(model_delta.clone()),
          _ => None,
        });

    let bounding_change_stream: Stream<BoxUpdate> = Default::default();
    let box_c = bounding_change_stream.clone();

    model_stream.on(move |model_delta| {
      if let Some(reactive) = weak_reactive.upgrade() {
        let mut reactive = reactive.write().unwrap();

        let box_c = box_c.clone();

        match model_delta {
          arena::ArenaDelta::Mutate((new_model, handle)) => {
            reactive[handle.into_raw_parts().0] =
              MeshBoxReactiveCache::from_model(new_model, *handle, box_c);
          }
          arena::ArenaDelta::Insert((model, handle)) => {
            box_c.emit(&BoxUpdate::Active(*handle));

            let r = MeshBoxReactiveCache::from_model(model, *handle, box_c);
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
