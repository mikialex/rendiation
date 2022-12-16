use crate::*;
use reactive::Stream;
use rendiation_geometry::Box3;

pub struct SceneBoundingSystem {
  /// actually data cache
  models_bounding: Vec<Box3>,

  /// mesh-model reverse mapping
  mesh_used_by_model: Arc<RwLock<HashMap<SceneMesh, MeshBoxReactiveCache>>>,

  update_queue: Vec<BoxUpdate>,

  /// for outside user subscribe
  bounding_change_stream: Stream<BoxUpdate>,
}

struct MeshBoxReactiveCache {
  local_box_stream: Stream<Option<Box3>>,
  models: Vec<(SceneModelHandle, Stream<Mat4<f32>>, Stream<SceneMesh>)>,
}

pub enum BoxUpdate {
  Remove(SceneModelHandle),
  Active(SceneModelHandle),
  Update { index: SceneModelHandle, bbox: Box3 },
}

impl SceneBoundingSystem {
  pub fn maintain(&mut self) {
    // self.update_queue;
  }

  pub fn new(scene: &Scene) -> Self {
    let mesh_used_by_model: Arc<RwLock<HashMap<SceneMesh, MeshBoxReactiveCache>>> =
      Default::default();
    let weak_mesh_used_by_model = Arc::downgrade(&mesh_used_by_model);

    let model_stream: Stream<&arena::ArenaDelta<SceneModel>> = scene
      .read()
      .delta_stream
      .filter_map(move |view| match view.delta {
        SceneInnerDelta::models(model_delta) => Some(model_delta),
        _ => None,
      });

    model_stream.on(move |model_delta| {
      if let Some(reactive) = weak_mesh_used_by_model.upgrade() {
        let reactive = reactive.write().unwrap();

        match model_delta {
          arena::ArenaDelta::Mutate(_) => todo!(),
          arena::ArenaDelta::Insert((model, _)) => {
            let model = model.read();
            let node = model.delta_stream().map().flatten();

            match &model.model {
              SceneModelType::Standard(model) => {
                let mesh_stream =
                  model
                    .read()
                    .delta_stream
                    .filter_map(move |view| match view.delta {
                      StandardModelDelta::mesh(mesh_delta) => Some(mesh_delta),
                      _ => None,
                    });

                let local_box_stream = mesh_stream.map(|mesh| mesh.read().compute_local_bound());
              }
              SceneModelType::Foreign(_) => todo!(),
            }
          }
          arena::ArenaDelta::Remove(handle) => {
            //
          }
        }

        false
      } else {
        true
      }
    });

    Self {
      models_bounding: Default::default(),
      mesh_used_by_model: Default::default(),
      update_queue: Default::default(),
      bounding_change_stream: Default::default(),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Box3 {
    &self.models_bounding[handle.into_raw_parts().0]
  }

  pub fn get_bounding_change_stream(&self) -> &Stream<BoxUpdate> {
    &self.bounding_change_stream
  }
}
