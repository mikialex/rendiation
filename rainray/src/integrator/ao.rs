use rendiation_algebra::{Vec3, Vector};
use rendiation_color::{Color, LinearRGBColorSpace};
use rendiation_geometry::Ray3;

use crate::{math::rand_point_in_unit_sphere, Scene};

use super::Integrator;

pub struct AOIntegrator {
  sample_count: u64,
}

impl Default for AOIntegrator {
  fn default() -> Self {
    Self { sample_count: 64 }
  }
}

fn sample_ao_surface(surface_point: Vec3<f32>, scene: &Scene) -> f32 {
  let test_ray =
    Ray3::from_point_to_point(surface_point, surface_point + rand_point_in_unit_sphere());
  if scene.get_any_hit(test_ray) {
    0.0
  } else {
    1.0
  }
}

impl Integrator for AOIntegrator {
  fn integrate(&self, scene: &Scene, ray: Ray3) -> Color<f32, LinearRGBColorSpace<f32>> {
    let ao_estimate = if let Some((intersection, _, _)) = scene.get_min_dist_hit(ray) {
      let mut ao_acc = 0.;
      for _ in 0..self.sample_count {
        ao_acc += sample_ao_surface(intersection.position, scene);
      }

      ao_acc / self.sample_count as f32
    } else {
      1.0
    };

    Color::new(Vec3::splat(ao_estimate))
  }
}
