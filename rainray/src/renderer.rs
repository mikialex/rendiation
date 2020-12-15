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
      integrator: Box::new(integrator),
    }
  }

  pub fn render(&mut self, camera: &Camera, scene: &Scene, frame: &mut Frame) {
    println!("rendering...");
    let now = Instant::now();
    // let mut render_frame = Frame::new(frame.width(), frame.height());

    let x_ratio_unit = 1.0 / frame.width() as f32;
    let y_ratio_unit = 1.0 / frame.width() as f32;

    let progress_bar = ProgressBar::new(100);
    let bar_inv = (frame.pixel_count() as f32 / 100.).ceil() as usize;
    let frame_size = frame.size();
    let width = frame.width();

    frame
      .data
      .par_iter_mut()
      .enumerate()
      .flat_map(|f| {
        let x = f.0;
        f.1.par_iter_mut().enumerate().map(move |i| ((x, i.0), i.1))
      })
      .for_each(|((i, j), pixel)| {
        *pixel = self
          .integrator
          .integrate(camera, scene, frame_size, (i, j).into());

        if (i * width + j) % bar_inv == 0 {
          progress_bar.inc(1);
        }
      });
    progress_bar.finish_and_clear();
    println!("frame data render finished.");

    let duration = now.elapsed();
    println!(
      "rendering used {} milliseconds.",
      duration.as_secs() * 1000 + u64::from(duration.subsec_millis())
    );
  }
}
