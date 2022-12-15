use crate::*;
use rendiation_geometry::Box3;

pub struct SceneBoundingSystem {
  models_bounding: Vec<Box3>,
  reactive_holder: Arc<RwLock<HashMap<SceneModelHandle, Box<dyn Any + Send + Sync>>>>,
}

impl SceneBoundingSystem {
  pub fn new(scene: &Scene) -> Self {
    let reactive_holder: Arc<RwLock<HashMap<SceneModelHandle, Box<dyn Any + Send + Sync>>>> =
      Default::default();
    let weak_reactive_holder = Arc::downgrade(&reactive_holder);
    let mapped = scene.read().delta_stream.on(move |view| {
      let reactive = weak_reactive_holder.upgrade().unwrap();
      let reactive = reactive.write();

      match view.delta {
        SceneInnerDelta::models(model_delta) => match model_delta {
          arena::ArenaDelta::Mutate((mutate, _)) => {
            //
          }
          arena::ArenaDelta::Insert(new_model) => {
            //
          }
          arena::ArenaDelta::Remove(model_handle) => {
            //
          }
        },
        _ => {}
      }
      false // todo
    });
    Self {
      models_bounding: Default::default(),
      reactive_holder,
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Box3 {
    &self.models_bounding[handle.into_raw_parts().0]
  }
}
