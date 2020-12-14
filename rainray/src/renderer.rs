use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use crate::{integrator::Integrator, scene::*};
use rendiation_render_entity::color::RGBColor;
use rendiation_render_entity::*;

use color::Color;
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::time::Instant;

pub struct Renderer {
  pub super_sample_rate: usize,

  pub integrator: Box<dyn Integrator>,
}

fn test_intersection_is_visible_to_point(
  scene: &Scene,
  intersection: &Intersection,
  point: &Vec3,
) -> bool {
  let distance = point.distance(intersection.hit_position);
  let test_ray = Ray3::from_point_to_point(intersection.hit_position, *point);
  let hit_result = scene.get_min_dist_hit(&test_ray);

  if let Some(hit_result) = hit_result {
    hit_result.0.distance > distance
  } else {
    true
  }
}

impl Renderer {
  pub fn new(integrator: impl Integrator + 'static) -> Renderer {
    Renderer {
      super_sample_rate: 1,
      integrator: Box::new(integrator),
    }
  }

  pub fn render(&mut self, camera: &Camera, scene: &Scene, frame: &mut Frame) {
    println!("rendering...");
    let now = Instant::now();
    let mut render_frame = Frame::new(
      frame.width() * self.super_sample_rate,
      frame.height() * self.super_sample_rate,
    );

    let x_ratio_unit = 1.0 / render_frame.width() as f32;
    let y_ratio_unit = 1.0 / render_frame.width() as f32;

    let progress_bar = ProgressBar::new(100);
    let bar_inv = (render_frame.pixel_count() as f32 / 100.).ceil() as usize;
    let width = frame.width();

    render_frame
      .data
      .par_iter_mut()
      .enumerate()
      .flat_map(|f| {
        let x = f.0;
        f.1.par_iter_mut().enumerate().map(move |i| ((x, i.0), i.1))
      })
      .for_each(|((i, j), pixel)| {
        let x_ratio = i as f32 * x_ratio_unit;
        let y_ratio = 1.0 - j as f32 * y_ratio_unit;

        *pixel = self
          .integrator
          .integrate(camera, scene, (x_ratio, y_ratio).into());

        if (i * width + j) % bar_inv == 0 {
          progress_bar.inc(1);
        }
      });
    progress_bar.finish_and_clear();
    println!("frame data render finished.");

    println!("down sample and output");

    let result_data = &mut frame.data;
    let super_sample_rate = self.super_sample_rate as usize;
    for (i, row) in result_data.iter_mut().enumerate() {
      for (j, pixel) in row.iter_mut().enumerate() {
        let super_sample_count = self.super_sample_rate as f32 * self.super_sample_rate as f32;
        let mut r_all = 0.0;
        let mut g_all = 0.0;
        let mut b_all = 0.0;
        for k in 0..super_sample_rate {
          for l in 0..super_sample_rate {
            let sample_pix =
              render_frame.data[i * super_sample_rate + k][j * super_sample_rate + l];
            let srgb = sample_pix.to_srgb();
            r_all += srgb.r();
            g_all += srgb.g();
            b_all += srgb.b();
          }
        }
        *pixel = Color::new(Vec3::new(r_all, g_all, b_all) / super_sample_count);
      }
    }

    let duration = now.elapsed();
    println!(
      "rendering used {} milliseconds.",
      duration.as_secs() * 1000 + u64::from(duration.subsec_millis())
    );
  }
}
