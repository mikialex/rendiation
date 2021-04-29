use rendiation_algebra::Vector;
use rendiation_color::{Color, LinearRGBColorSpace};
use rendiation_geometry::Ray3;

use crate::{math::rand_point_in_unit_sphere, math::Vec3, RayTraceScene};

use super::Integrator;

pub struct AOIntegrator {
  sample_count: u64,
}

impl Default for AOIntegrator {
  fn default() -> Self {
    Self { sample_count: 100 }
  }
}

fn sample_ao_surface<'a>(surface_point: Vec3, scene: &RayTraceScene<'a>) -> f32 {
  let test_ray =
    Ray3::from_point_to_point(surface_point, surface_point + rand_point_in_unit_sphere());
  if scene.get_min_dist_hit(test_ray).is_some() {
    0.0
  } else {
    1.0
  }
}

impl Integrator for AOIntegrator {
  fn integrate<'a>(
    &self,
    scene: &RayTraceScene<'a>,
    ray: Ray3,
  ) -> Color<f32, LinearRGBColorSpace<f32>> {
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
