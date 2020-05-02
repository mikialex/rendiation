use rendiation_math_entity::Frustum;
use rendiation_render_entity::Camera;
use super::{scene::Scene, render_list::RenderList};

pub struct Culler {
  frustum: Frustum,
  enable_frustum_culling: bool,
}

impl Culler {
  pub fn update(&mut self, camera: &impl Camera) -> &mut Self {
    let m = camera.get_vp_matrix();
    self.frustum.set_from_matrix(m);
    self
  }

  pub fn execute_culling(&self, render_list: &mut RenderList, scene: &Scene){
    if self.enable_frustum_culling {
      todo!()
    }
  }
}
