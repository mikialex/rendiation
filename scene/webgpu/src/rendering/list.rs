use crate::*;

pub struct RenderList {
  pub(crate) opaque: Vec<(SceneModelHandle, f32)>,
  pub(crate) transparent: Vec<(SceneModelHandle, f32)>,
}

impl Default for RenderList {
  fn default() -> Self {
    Self {
      opaque: Vec::new(),
      transparent: Vec::new(),
    }
  }
}

impl RenderList {
  pub fn prepare(&mut self, scene: &Scene, camera: &SceneCamera) {
    if scene.active_camera.is_none() {
      return;
    }

    let camera_mat = camera.visit(|camera| camera.node.get_world_matrix());
    let camera_pos = camera_mat.position();
    let camera_forward = camera_mat.forward() * -1.;

    self.opaque.clear();
    self.transparent.clear();

    for (h, m) in scene.models.iter() {
      let model_pos = m.get_node().get_world_matrix().position();
      let depth = (model_pos - camera_pos).dot(camera_forward);

      let is_transparent = (m.deref() as &dyn SceneModelShareable).is_transparent();
      if is_transparent {
        self.transparent.push((h, depth));
      } else {
        self.opaque.push((h, depth));
      }
    }

    self.opaque.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    self
      .transparent
      .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
  }

  pub fn setup_pass(
    &self,
    gpu_pass: &mut SceneRenderPass,
    scene: &Scene,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.opaque.iter().for_each(|(handle, _)| {
      let model = scene.models.get(*handle).unwrap();
      model.render(gpu_pass, dispatcher, camera)
    });
    self.transparent.iter().for_each(|(handle, _)| {
      let model = scene.models.get(*handle).unwrap();
      model.render(gpu_pass, dispatcher, camera)
    });
  }
}
