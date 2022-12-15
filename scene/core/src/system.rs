use crate::*;
use rendiation_geometry::Box3;

struct SceneBoundingSystem {
  models_bounding: Vec<Box3>,
  reactive_holder: HashMap<SceneModelHandle, Box<dyn Any>>,
}

impl SceneBoundingSystem {
  pub fn new(scene: &Scene) -> Self {
    let mapped = scene.read().delta_stream.filter_map(|view| {
      match view.delta {
        SceneInnerDelta::models(model_delta) => match model_delta {
          arena::ArenaDelta::Mutate(mutate) => todo!(),
          arena::ArenaDelta::Insert(new_model) => {
            //
          }
          arena::ArenaDelta::Remove(model_handle) => {
            //
          }
        },
        _ => {}
      }
    });
    Self {
      models_bounding: Default::default(),
      reactive_holder: Box::new(mapped),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Box3 {
    &self.models_bounding[handle.into_raw_parts().0]
  }
}
