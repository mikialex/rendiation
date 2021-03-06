use crate::*;
use rendiation_algebra::{Vec2, Vec3, Vector};

use indicatif::ProgressBar;
use rayon::prelude::*;
use rendiation_color::Color;
use std::time::Instant;

pub struct Renderer {
  pub sample_per_pixel: usize,
  pub integrator: Box<dyn Integrator>,
}

impl Renderer {
  pub fn new(integrator: impl Integrator + 'static) -> Renderer {
    Renderer {
      sample_per_pixel: integrator.default_sample_per_pixel(),
      integrator: Box::new(integrator),
    }
  }

  pub fn render(&mut self, camera: &Camera, scene: &mut Scene, frame: &mut Frame) {
    scene.update();

    println!("rendering...");
    let now = Instant::now();

    let progress_bar = ProgressBar::new(100);
    let bar_inv = (frame.pixel_count() as f32 / 100.).ceil() as usize;
    let frame_size = frame.size().map(|v| v as f32);
    let jitter_unit = frame_size.map(|v| 1. / v);
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
        let x = i as f32 / frame_size.x;
        let y = (frame_size.y - j as f32) / frame_size.y;

        let mut energy_acc = Vec3::zero();

        for _ in 0..self.sample_per_pixel {
          let sample_point = Vec2::new(x, y) + jitter_unit.map(|v| v * rand());
          let ray = camera.create_screen_ray(sample_point);
          energy_acc += self.integrator.integrate(&scene, ray).value;
        }

        energy_acc /= self.sample_per_pixel as f32;
        *pixel = Color::new(energy_acc);

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
