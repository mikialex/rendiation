use std::time::Instant;

use indicatif::ProgressBar;
use rayon::prelude::*;
use rendiation_algebra::*;
use rendiation_color::LinearRGBColor;
use rendiation_geometry::{Ray3, RayCaster3};
use rendiation_texture::Texture2D;

pub mod ao;
pub mod intersection_stat;
pub mod path_trace;
pub use ao::*;
pub use intersection_stat::*;
pub use path_trace::*;

use crate::Frame;
use crate::*;

pub trait Integrator<T: Send + Sync>: Send + Sync {
  fn integrate(&self, target: &T, ray: Ray3) -> LinearRGBColor<f32>;

  fn render(
    &mut self,
    ray_source: &(impl RayCaster3<f32> + Send + Sync),
    scene: &mut T,
    frame: &mut Frame,
    sample_per_pixel: usize,
  ) {
    println!("rendering...");
    let now = Instant::now();

    let progress_bar = ProgressBar::new(100);
    let bar_inv = (frame.pixel_count() as f32 / 100.).ceil() as usize;
    let frame_size = frame.size().map(|v| v as f32);
    let jitter_unit = frame_size.map(|v| 1. / v);
    let height = frame.height();

    frame
      .inner
      .iter_mut()
      .par_bridge()
      .for_each(|(pixel, (i, j))| {
        let x = i as f32 / frame_size.x;
        let y = (frame_size.y - j as f32) / frame_size.y;

        let mut energy_acc = Vec3::zero();

        for _ in 0..sample_per_pixel {
          let sample_point = Vec2::new(x, y) + jitter_unit.map(|v| v * rand());
          let sample_point = sample_point * 2. - Vec2::one();
          let ray = ray_source.cast_ray(sample_point);
          energy_acc += self.integrate(scene, ray).into();
        }

        energy_acc /= sample_per_pixel as f32;
        *pixel = energy_acc.into();

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

pub trait RayTraceable: Send + Sync {
  fn get_any_hit(&self, world_ray: Ray3) -> bool;
  fn get_min_dist_hit_stat(&self, world_ray: Ray3) -> IntersectionStatistic;
  fn get_min_dist_hit(&self, world_ray: Ray3) -> Option<(Intersection, f32, &Model)>;
  fn test_point_visible_to_point(&self, point_a: Vec3<f32>, point_b: Vec3<f32>) -> bool;
}
