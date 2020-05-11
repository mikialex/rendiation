use rendiation_math_entity::Frustum;
use rendiation_render_entity::Camera;
use super::{scene::Scene};
use crate::Index;

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

  pub fn test_is_visible(&self, node_id: Index, scene: &Scene) -> bool{
    let render_data = scene.nodes_render_data.get(node_id).unwrap();
    if self.enable_frustum_culling {
      if let Some(bounding) = &render_data.world_bounding {
        if !bounding.if_intersect_frustum(&self.frustum){
          return false
        }
      }
    }
    true
  }

}
