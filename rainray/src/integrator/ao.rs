use rendiation_math::Vec2;
use rendiation_math_entity::Ray3;
use rendiation_render_entity::{
  color::{Color, LinearRGBColorSpace},
  Camera, Raycaster,
};

use crate::{math::rand_point_in_unit_sphere, math::Vec3, scene::Scene};

use super::Integrator;

pub struct AOIntegrator {
  sample_count: u64,
}

impl AOIntegrator {
  pub fn new() -> Self {
    Self { sample_count: 100 }
  }

  fn sample_ao(&self, ray: &Ray3, scene: &Scene) -> f32 {
    let hit_result = scene.get_min_dist_hit(ray);

    if let Some((intersection, _)) = scene.get_min_dist_hit(ray) {
      let mut ao_acc = 0.;
      for _ in 0..self.sample_count {
        ao_acc += sample_ao_surface(intersection.hit_position, scene);
      }

      ao_acc / self.sample_count as f32
    } else {
      1.0
    }
  }
}

fn sample_ao_surface(surface_point: Vec3, scene: &Scene) -> f32 {
  let test_ray =
    Ray3::from_point_to_point(surface_point, surface_point + rand_point_in_unit_sphere());
  if scene.get_min_dist_hit(&test_ray).is_some() {
    0.0
  } else {
    1.0
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
    Color::new(Vec3::splat(self.sample_ao(&ray, scene)))
  }
}
