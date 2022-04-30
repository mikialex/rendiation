use std::cmp::Ordering;

use rendiation_algebra::*;
use rendiation_geometry::Nearest;
use rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig;

use crate::*;

impl Scene {
  pub fn pick_nearest(
    &self,
    normalized_position: Vec2<f32>,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<&dyn SceneRenderableShareable> {
    let mut result = Vec::new();

    let camera = self.active_camera.as_ref().unwrap();
    let camera_world_mat = camera.node.visit(|n| n.world_matrix);
    let world_ray = camera
      .cast_world_ray(normalized_position)
      .apply_matrix_into(camera_world_mat);

    for m in self.models.iter() {
      if let Some(Nearest(Some(r))) = m.ray_pick_nearest(&world_ray, conf) {
        println!("pick");
        result.push((m, r));
      }
    }

    result.sort_by(|(_, a), (_, b)| {
      a.hit
        .distance
        .partial_cmp(&b.hit.distance)
        .unwrap_or(Ordering::Less)
    });

    result.first().map(|r| r.0.as_ref())
  }
}
