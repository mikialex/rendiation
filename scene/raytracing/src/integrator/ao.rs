use rendiation_algebra::{Vec3, Vector};
use rendiation_color::LinearRGBColor;
use rendiation_geometry::Ray3;

use super::Integrator;
use crate::{
  uniform_sample_sphere_dir, FixedSamplesPerPixel, RayTraceContentBase, RngSampler, Sampler,
};

pub struct AOIntegrator {
  sample_count: usize,
}

impl Default for AOIntegrator {
  fn default() -> Self {
    Self { sample_count: 64 }
  }
}

fn sample_ao_surface(
  surface_point: Vec3<f32>,
  target: &impl RayTraceContentBase,
  sampler: &mut dyn Sampler,
) -> f32 {
  let test_ray = Ray3::new(
    surface_point,
    uniform_sample_sphere_dir(sampler.next_vec2()),
  );
  if target.get_any_hit(test_ray) {
    0.0
  } else {
    1.0
  }
}

impl<T: RayTraceContentBase> Integrator<T> for AOIntegrator {
  type PixelSampler = FixedSamplesPerPixel;
  fn create_pixel_sampler(&self) -> Self::PixelSampler {
    FixedSamplesPerPixel::by_target_samples_per_pixel(self.sample_count)
  }

  type Sampler = RngSampler;
  fn create_sampler(&self) -> Self::Sampler {
    Default::default()
  }

  fn integrate(&self, target: &T, ray: Ray3, sampler: &mut dyn Sampler) -> LinearRGBColor<f32> {
    let ao_estimate = if let Some((intersection, _)) = target.get_min_dist_hit(ray) {
      let mut ao_acc = 0.;
      for _ in 0..self.sample_count {
        ao_acc += sample_ao_surface(intersection.position, target, sampler);
      }

      ao_acc / self.sample_count as f32
    } else {
      1.0
    };

    Vec3::splat(ao_estimate).into()
  }
}
