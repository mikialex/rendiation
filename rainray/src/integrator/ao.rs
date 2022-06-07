use rendiation_algebra::{Vec3, Vector};
use rendiation_color::LinearRGBColor;
use rendiation_geometry::Ray3;

use crate::{math::rand_point_in_unit_sphere, FixedSamplesPerPixel, RayTraceable};

use super::Integrator;

pub struct AOIntegrator {
  sample_count: usize,
}

impl Default for AOIntegrator {
  fn default() -> Self {
    Self { sample_count: 64 }
  }
}

fn sample_ao_surface<T: RayTraceable>(surface_point: Vec3<f32>, target: &T) -> f32 {
  let test_ray =
    Ray3::from_point_to_point(surface_point, surface_point + rand_point_in_unit_sphere());
  if target.get_any_hit(test_ray) {
    0.0
  } else {
    1.0
  }
}

impl<T: RayTraceable> Integrator<T> for AOIntegrator {
  type PixelSampler = FixedSamplesPerPixel;
  fn create_pixel_sampler(&self) -> Self::PixelSampler {
    FixedSamplesPerPixel::by_target_samples_per_pixel(self.sample_count)
  }
  fn integrate(&self, target: &T, ray: Ray3) -> LinearRGBColor<f32> {
    let ao_estimate = if let Some((intersection, _, _)) = target.get_min_dist_hit(ray) {
      let mut ao_acc = 0.;
      for _ in 0..self.sample_count {
        ao_acc += sample_ao_surface(intersection.position, target);
      }

      ao_acc / self.sample_count as f32
    } else {
      1.0
    };

    Vec3::splat(ao_estimate).into()
  }
}
