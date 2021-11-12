use rendiation_algebra::*;

use crate::*;

impl Scene {
  pub fn pick(&self, normalized_position: Vec2<f32>) -> Vec<&MeshModel> {
    let result = Vec::new();

    let camera = self.active_camera.as_ref().unwrap();
    let view_mat = camera.node.visit(|n| n.world_matrix).inverse_or_identity();
    let world_ray = camera
      .cast_world_ray(normalized_position)
      .apply_matrix_into(view_mat);

    self.models.iter().for_each(|model| {
      let model = model.inner.borrow();
      let world_inv = model.node.visit(|n| n.world_matrix).inverse_or_identity();
      let local_ray = world_ray.clone().apply_matrix(world_inv);

      // todo or move this part into model impl
    });

    result
  }
}
