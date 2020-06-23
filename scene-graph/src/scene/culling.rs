use super::scene::Scene;
use crate::{SceneGraphBackend, SceneNodeHandle};
use rendiation_math_entity::{Frustum, IntersectAble};
use rendiation_render_entity::Camera;

pub struct Culler {
  frustum: Frustum,
  pub enable_frustum_culling: bool,
}

impl Culler {
  pub fn new() -> Self {
    Self {
      frustum: Frustum::new(),
      enable_frustum_culling: true,
    }
  }

  pub fn update(&mut self, camera: &impl Camera) -> &mut Self {
    let m = camera.get_vp_matrix();
    self.frustum.set_from_matrix(m);
    self
  }

  pub fn test_is_visible<T: SceneGraphBackend>(
    &self,
    node_id: SceneNodeHandle<T>,
    scene: &Scene<T>,
  ) -> bool {
    let render_data = scene.get_node_render_data(node_id);
    if self.enable_frustum_culling {
      if let Some(bounding) = &render_data.world_bounding {
        if !bounding.intersect(&self.frustum, &()) {
          return false;
        }
      }
    }
    true
  }
}
