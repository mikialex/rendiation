use rendiation_geometry::Box3;
struct SceneBoundingSystem {
  models_bounding: Vec<Box3>,
}

impl SceneBoundingSystem {
  pub fn new(scene: &Scene) -> Self {
    scene.read().delta_stream.filter_map(|delta| {
      //
    });
    Self {
      models_bounding: Default::default(),
    }
  }

  pub fn get_model_bounding(&self, handle: SceneModelHandle) -> &Box3 {
    self.models_bounding[handle.index]
  }
}
