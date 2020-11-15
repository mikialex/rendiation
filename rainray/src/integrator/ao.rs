use rendiation_math::Vec2;
use rendiation_math_entity::Ray3;
use rendiation_render_entity::{
  color::{Color, LinearRGBColorSpace},
  Camera, Raycaster,
};

use crate::{math::Vec3, scene::Scene};

use super::Integrator;

pub struct AOIntegrator {
  sample_count: u64,
}

impl AOIntegrator {
  fn sample_ao(&self, ray: &Ray3, scene: &Scene) -> f32 {
    todo!()
  }
}

impl Integrator for AOIntegrator {
  fn integrate(
    &self,
    camera: &Camera,
    scene: &Scene,
    view_position: Vec2<f32>,
  ) -> Color<LinearRGBColorSpace<f32>> {
    let ray = camera.create_screen_ray(view_position);

    let mut ao_acc = 0.;

    for _ in 0..self.sample_count {
      ao_acc += self.sample_ao(&ray, scene);
    }

    let max = self.sample_count as f32;
    Color::new(Vec3::splat(ao_acc / max))
  }
}
