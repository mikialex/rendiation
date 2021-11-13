use rendiation_algebra::*;
use rendiation_renderable_mesh::mesh::{IntersectAbleGroupedMesh, MeshBufferIntersectConfig};

use crate::*;

impl Scene {
  pub fn pick(&self, normalized_position: Vec2<f32>) -> Vec<&MeshModel> {
    let mut result = Vec::new();

    let camera = self.active_camera.as_ref().unwrap();
    let view_mat = camera.node.visit(|n| n.world_matrix).inverse_or_identity();
    let world_ray = camera
      .cast_world_ray(normalized_position)
      .apply_matrix_into(view_mat);

    let conf = MeshBufferIntersectConfig::default();

    for m in self.models.iter() {
      let model = m.inner.borrow();
      let world_inv = model.node.visit(|n| n.world_matrix).inverse_or_identity(); // todo support view scale mesh

      let local_ray = world_ray.clone().apply_matrix_into(world_inv);

      if !model.material.is_keep_mesh_shape() {
        continue;
      }

      let mesh = &model.mesh;
      mesh.try_pick(&mut |mesh: &dyn IntersectAbleGroupedMesh| {
        if mesh
          .intersect_nearest(local_ray, &conf, model.group)
          .is_some()
        {
          result.push(m);
        }
      });
    }

    result
  }
}
