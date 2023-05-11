use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) opaque: Vec<(SceneModelHandle, f32)>,
  pub(crate) transparent: Vec<(SceneModelHandle, f32)>,
}

pub fn is_model_enable_blend(model: &ModelType) -> bool {
  match model {
    ModelType::Standard(model) => model.read().material.is_transparent(),
    ModelType::Foreign(_) => false, // todo
    _ => false,
  }
}

impl RenderList {
  pub fn prepare(
    &mut self,
    scene: &SceneInner,
    camera: &SceneCamera,
    node_derives: &SceneNodeDeriveSystem,
  ) {
    if scene.active_camera.is_none() {
      return;
    }

    let camera_mat = camera.visit(|camera| node_derives.get_world_matrix(&camera.node));
    let camera_pos = camera_mat.position();
    let camera_forward = camera_mat.forward().reverse();

    self.opaque.clear();
    self.transparent.clear();

    for (h, m) in scene.models.iter() {
      let model_pos = node_derives.get_world_matrix(&m.get_node()).position();
      let depth = (model_pos - camera_pos).dot(camera_forward);

      let is_transparent = is_model_enable_blend(&m.read().model);
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
    scene: &SceneInner,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    let models = &scene.models;
    self.opaque.iter().for_each(|(handle, _)| {
      let model = models.get(*handle).unwrap();
      model.render(gpu_pass, dispatcher, camera)
    });
    self.transparent.iter().for_each(|(handle, _)| {
      let model = models.get(*handle).unwrap();
      model.render(gpu_pass, dispatcher, camera)
    });
  }
}
