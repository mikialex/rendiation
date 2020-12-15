use rendiation_math::Vec2;
use rendiation_math_entity::Ray3;
use rendiation_render_entity::{
  color::{Color, LinearRGBColorSpace},
  Camera, Raycaster,
};

use crate::{math::rand, math::rand_point_in_unit_sphere, math::Vec3, scene::Scene};

use super::Integrator;

pub struct AOIntegrator {
  sample_count: u64,
}

impl Default for AOIntegrator {
  fn default() -> Self {
    Self { sample_count: 100 }
  }
}

impl AOIntegrator {
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
    frame_size: Vec2<usize>,
    current: Vec2<usize>,
  ) -> Color<LinearRGBColorSpace<f32>> {
    let mut pixel_left_top = current.map(|v| v as f32) / frame_size.map(|v| v as f32);
    pixel_left_top.y = 1.0 - pixel_left_top.y;
    let ray = camera.create_screen_ray(pixel_left_top);

    Color::new(Vec3::splat(self.sample_ao(&ray, scene)))
  }
}
