use std::cmp::Ordering;

use rendiation_algebra::*;
use rendiation_geometry::Nearest;
use rendiation_renderable_mesh::mesh::{IntersectAbleGroupedMesh, MeshBufferIntersectConfig};

use crate::*;

impl Scene {
  pub fn pick_nearest(
    &self,
    normalized_position: Vec2<f32>,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<&MeshModel> {
    let mut result = Vec::new();

    let camera = self.active_camera.as_ref().unwrap();
    let camera_world_mat = camera.node.visit(|n| n.world_matrix);
    let world_ray = camera
      .cast_world_ray(normalized_position)
      .apply_matrix_into(camera_world_mat);

    for m in self.models.iter() {
      let model = m.inner.borrow();
      let world_inv = model.node.visit(|n| n.world_matrix).inverse_or_identity(); // todo support view scale mesh

      let local_ray = world_ray.clone().apply_matrix_into(world_inv);

      if !model.material.is_keep_mesh_shape() {
        continue;
      }

      let mesh = &model.mesh;
      mesh.try_pick(&mut |mesh: &dyn IntersectAbleGroupedMesh| {
        if let Nearest(Some(r)) = mesh.intersect_nearest(local_ray, conf, model.group) {
          println!("pick");
          result.push((m, r));
        }
      });
    }

    result.sort_by(|(_, a), (_, b)| {
      a.hit
        .distance
        .partial_cmp(&b.hit.distance)
        .unwrap_or(Ordering::Less)
    });

    result.first().map(|r| r.0)
  }
}
