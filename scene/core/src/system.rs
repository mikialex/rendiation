use crate::*;
use reactive::Stream;
use rendiation_geometry::Box3;

pub struct SceneBoundingSystem {
  /// actually data cache
  models_bounding: Vec<Box3>,

  reactive: Arc<RwLock<Vec<Option<MeshBoxReactiveCache>>>>,

  update_queue: Vec<BoxUpdate>,

  /// for outside user subscribe
  bounding_change_stream: Stream<BoxUpdate>,
}

struct MeshBoxReactiveCache {
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
    let world_mat_stream = model
      .delta_stream
      .filter_map(|node| match node.delta {
        SceneModelImplDelta::model(_) => None,
        SceneModelImplDelta::node(node) => Some(node),
      })
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
    // self.update_queue;
  }

  pub fn new(scene: &Scene) -> Self {
    let reactive: Arc<RwLock<Vec<Option<MeshBoxReactiveCache>>>> = Default::default();
    let weak_reactive = Arc::downgrade(&reactive);

    let model_stream: Stream<&arena::ArenaDelta<SceneModel>> = scene
      .read()
      .delta_stream
      .filter_map(move |view| match view.delta {
        SceneInnerDelta::models(model_delta) => Some(model_delta),
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
            reactive[handle.into_raw_parts().0] =
              MeshBoxReactiveCache::from_model(model, *handle, box_c);
          }
          arena::ArenaDelta::Remove(handle) => {
            reactive[handle.into_raw_parts().0] = None;
          }
        }

        false
      } else {
        true
      }
    });

    Self {
      models_bounding: Default::default(),
      reactive: Default::default(),
      update_queue: Default::default(),
      bounding_change_stream,
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Box3 {
    &self.models_bounding[handle.into_raw_parts().0]
  }

  pub fn get_bounding_change_stream(&self) -> &Stream<BoxUpdate> {
    &self.bounding_change_stream
  }
}
