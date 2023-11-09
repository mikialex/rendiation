use std::time::Instant;

use indicatif::ProgressBar;
pub use pixel_sample_control::*;
use rayon::prelude::*;
use rendiation_algebra::*;
use rendiation_color::LinearRGBColor;
use rendiation_geometry::{Ray3, RayCaster3};
use rendiation_texture::{Texture2D, Texture2DBuffer};

mod ao;
mod intersection_stat;
mod path_trace;
mod pixel_sample_control;
pub use ao::*;
pub use intersection_stat::*;
pub use path_trace::*;

use crate::*;

pub trait Integrator<T: Send + Sync>: Send + Sync {
  /// per pixel integrate logic
  fn integrate(&self, target: &T, ray: Ray3, sampler: &mut dyn Sampler) -> LinearRGBColor<f32>;

  /// per pixel sampler controller this integrator prefer
  type PixelSampler: PixelSampleController;
  fn create_pixel_sampler(&self) -> Self::PixelSampler;

  /// the actual sample provider this integrator prefer
  type Sampler: Sampler;
  fn create_sampler(&self) -> Self::Sampler;

  /// the render logic
  fn render(
    &mut self,
    ray_source: &(impl RayCaster3<f32> + Send + Sync),
    scene: &mut T,
    frame: &mut Texture2DBuffer<LinearRGBColor<f32>>,
  ) {
    println!("rendering...");
    let now = Instant::now();

    let progress_bar = ProgressBar::new(100);
    let bar_inv = (frame.pixel_count() as f32 / 100.).ceil() as usize;
    let frame_size: Vec2<_> = frame.size().into_f32().into();
    let jitter_unit = frame_size.map(|v| 1. / v);
    let height = frame.height();

    frame.iter_mut().par_bridge().for_each(|(pixel, (i, j))| {
      let x = i as f32 / frame_size.x;
      let y = (frame_size.y - j as f32) / frame_size.y;

      let mut sampler = self.create_sampler();

      *pixel = self
        .create_pixel_sampler()
        .sample_pixel(|next_sampling_index| {
          sampler.reset(next_sampling_index);

          let sample_point = Vec2::new(x, y) + jitter_unit * sampler.next_vec2();
          let sample_point = sample_point * 2. - Vec2::one();
          let ray = ray_source.cast_ray(sample_point);
          self.integrate(scene, ray, &mut sampler).into()
        })
        .into();

      if (i + j * height) % bar_inv == 0 {
        progress_bar.inc(1);
      }
    });
    progress_bar.finish_and_clear();
    println!("frame data render finished.");

    let duration = now.elapsed();
    println!(
      "rendering used {} milliseconds.",
      duration.as_secs() * 1000 + <u64 as From<_>>::from(duration.subsec_millis())
    );
  }
}

/// different integrator has different requirements for the target(scene), this is the base trait
pub trait RayTraceContentBase: Send + Sync {
  fn get_any_hit(&self, world_ray: Ray3) -> bool;
  fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32)>;

  fn test_point_visible_to_point(&self, point_a: Vec3<f32>, point_b: Vec3<f32>) -> bool {
    let ray = Ray3::from_point_to_point(point_a, point_b);
    let distance = (point_a - point_b).length();

    if let Some(hit_result) = self.get_min_dist_hit(ray) {
      hit_result.1 > distance
    } else {
      true
    }
  }
}
