use rendiation_color::LinearRGBColor;
use rendiation_geometry::Ray3;

use crate::*;

pub struct IntersectionVisualize {
  pub box_weight: f32,
  pub sphere_weight: f32,
  pub triangle_weight: f32,

  pub weight_bound: f32,
}

impl Default for IntersectionVisualize {
  fn default() -> Self {
    Self {
      box_weight: 1.,
      sphere_weight: 1.,
      triangle_weight: 1.,
      weight_bound: 150.,
    }
  }
}

pub trait RayTraceContentForStat: RayTraceContentBase {
  fn get_min_dist_hit_stat(&self, world_ray: Ray3) -> IntersectionStatistic;
}

impl<T> Integrator<T> for IntersectionVisualize
where
  T: RayTraceContentForStat,
{
  type PixelSampler = FixedSamplesPerPixel;
  fn create_pixel_sampler(&self) -> Self::PixelSampler {
    FixedSamplesPerPixel::by_target_samples_per_pixel(4)
  }

  type Sampler = RngSampler;
  fn create_sampler(&self) -> Self::Sampler {
    Default::default()
  }

  fn integrate(&self, target: &T, ray: Ray3, _: &mut dyn Sampler) -> LinearRGBColor<f32> {
    let stat = target.get_min_dist_hit_stat(ray);
    let cost_estimate = self.box_weight * stat.box3 as f32
      + self.sphere_weight * stat.sphere as f32
      + self.triangle_weight * stat.triangle as f32;

    LinearRGBColor::splat(cost_estimate / self.weight_bound)
  }
}
