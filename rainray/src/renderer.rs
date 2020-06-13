use crate::frame::*;
use crate::math::*;
use crate::ray::*;
use crate::scene::*;
use rendiation_render_entity::color::RGBColor;
use rendiation_render_entity::*;

use indicatif::ProgressBar;
use std::time::Instant;

pub struct Renderer {
  super_sample_rate: u64,
  exposure_upper_bound: f32,

  trace_fix_sample_count: u64,
  bounce_time_limit: u64,
}

fn test_intersection_is_visible_to_point(
  scene: &Scene,
  intersection: &Intersection,
  point: &Vec3,
) -> bool {
  let distance = (*point - intersection.hit_position).length();
  let test_ray = Ray3::from_point_to_point(intersection.hit_position, *point);
  let hit_result = scene.get_min_dist_hit(&test_ray);

  if let Some(hit_result) = hit_result {
    hit_result.0.distance > distance
  } else {
    true
  }
}

impl Renderer {
  pub fn new() -> Renderer {
    let super_sample_rate = 1;
    Renderer {
      super_sample_rate,
      exposure_upper_bound: 1.0,
      bounce_time_limit: 10,
      trace_fix_sample_count: 10,
    }
  }

  pub fn path_trace(&self, ray: &Ray3, scene: &Scene, _camera: &impl Camera) -> Vec3 {
    let mut energy = Vec3::new(0., 0., 0.);
    let mut throughput = Vec3::new(1., 1., 1.);
    let mut current_ray = *ray;

    for _depth in 0..self.bounce_time_limit {
      let hit_result = scene.get_min_dist_hit(&current_ray);

      if hit_result.is_none() {
        energy += scene.env.sample(&current_ray) * throughput;
        break;
      }
      let (intersection, model) = hit_result.unwrap();
      let material = model.material;

      energy += material.collect_energy(&current_ray) * throughput;

      let next_ray = Ray3::from_point_to_point(
        intersection.hit_position,
        intersection.hit_position
          + intersection.hit_normal
          + rand_point_in_unit_sphere(),
      );

      let brdf = model
        .material
        .brdf(&intersection, &current_ray, &next_ray);

      let pdf =
        model
          .material
          .brdf_importance_pdf(&intersection, &current_ray, &next_ray);

      throughput = throughput * brdf / pdf;

      current_ray = next_ray;
    }

    energy
  }

  pub fn render(&self, camera: &PerspectiveCamera, scene: &Scene, frame: &mut Frame) {
    println!("start render");
    let now = Instant::now();
    let mut render_frame = Frame::new(
      frame.width * self.super_sample_rate,
      frame.height * self.super_sample_rate,
    );

    let x_ratio_unit = 1.0 / render_frame.width as f32;
    let y_ratio_unit = 1.0 / render_frame.width as f32;
    let energy_div = self.trace_fix_sample_count as f32 * self.exposure_upper_bound;

    let progress_bar = ProgressBar::new(100);
    let bar_inv = (render_frame.width as f32 / 100.).ceil() as usize;

    for (i, row) in render_frame.data.iter_mut().enumerate() {
      for (j, pixel) in row.iter_mut().enumerate() {
        let x_ratio = i as f32 * x_ratio_unit;
        let y_ratio = 1.0 - j as f32 * y_ratio_unit;
        let ray = camera.create_screen_ray((x_ratio, y_ratio).into());

        let mut energy_acc = Vec3::new(0., 0., 0.);

        for _sample in 0..self.trace_fix_sample_count {
          energy_acc += self.path_trace(&ray, scene, camera);
        }
        pixel
          .mut_r(energy_acc.x / energy_div)
          .mut_g(energy_acc.y / energy_div)
          .mut_b(energy_acc.z / energy_div);
      }
      if i % bar_inv == 0 {
        progress_bar.inc(1);
      }
    }
    progress_bar.finish_and_clear();
    println!("frame data render finished.");

    println!("start super sample down sample and gamma correction");

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
        pixel
          .mut_r(r_all / super_sample_count)
          .mut_g(g_all / super_sample_count)
          .mut_b(b_all / super_sample_count);
      }
    }

    let duration = now.elapsed();
    println!(
      "rendering used {} milliseconds.",
      duration.as_secs() * 1000 + u64::from(duration.subsec_millis())
    );
  }
}
