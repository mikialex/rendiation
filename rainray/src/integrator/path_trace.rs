use rendiation_algebra::InnerProductSpace;
use rendiation_color::{Color, LinearRGBColorSpace};
use rendiation_geometry::Ray3;

use super::Integrator;
use crate::{
  math::rand, math::Vec3, scene::Scene, Intersection, LightSampleResult, Material, NormalizedVec3,
};
use rendiation_algebra::RealVector;

pub struct PathTraceIntegrator {
  pub exposure_upper_bound: f32,
  pub trace_fix_sample_count: u64,
  pub bounce_time_limit: u64,
  pub roulette_threshold: f32,
  pub roulette_factor: f32,
}

impl Default for PathTraceIntegrator {
  fn default() -> Self {
    Self {
      exposure_upper_bound: 1.0,
      bounce_time_limit: 20,
      trace_fix_sample_count: 200,
      roulette_threshold: 0.05,
      roulette_factor: 0.05,
    }
  }
}

impl PathTraceIntegrator {
  // next event estimation
  fn sample_lights(
    &self,
    scene: &Scene,
    material: &dyn Material,
    intersection: &Intersection,
    light_out_dir: NormalizedVec3,
  ) -> Vec3 {
    let mut energy = Vec3::new(0.0, 0.0, 0.0);
    for light in &scene.lights {
      if let Some(LightSampleResult {
        emissive,
        light_in_dir,
      }) = light.sample(intersection.position, scene)
      {
        let bsdf = material.bsdf(light_in_dir.reverse(), light_out_dir, intersection);
        energy += bsdf * emissive * -light_in_dir.dot(intersection.geometric_normal);
      }
    }
    energy
  }
}

impl Integrator for PathTraceIntegrator {
  fn integrate(&self, scene: &Scene, ray: Ray3) -> Color<f32, LinearRGBColorSpace<f32>> {
    let mut energy = Vec3::new(0., 0., 0.);
    let mut throughput = Vec3::new(1., 1., 1.);
    let mut current_ray = ray;

    for _depth in 0..self.bounce_time_limit {
      let hit_result = scene.get_min_dist_hit(current_ray);

      // hit outside scene, sample background;
      if hit_result.is_none() {
        energy += scene.env.sample(&current_ray) * throughput;
        break;
      }

      let (intersection, model) = hit_result.unwrap();
      let material = &model.material;

      let view_dir = current_ray.direction.reverse();
      let light_dir = material.sample_light_dir(view_dir, &intersection);
      let light_dir_pdf = material.pdf(view_dir, light_dir, &intersection);
      if light_dir_pdf == 0.0 {
        break;
      }

      energy += self.sample_lights(
        scene,
        material.as_ref(),
        &intersection,
        current_ray.direction.reverse(),
      ) * throughput;

      let cos = light_dir.dot(intersection.geometric_normal).abs();
      let bsdf = material.bsdf(view_dir, light_dir, &intersection);
      throughput = throughput * cos * bsdf / light_dir_pdf;

      // roulette exist
      if throughput.max_channel() < self.roulette_threshold {
        if rand() < self.roulette_factor {
          break;
        }
        throughput /= 1. - self.roulette_factor;
      }

      current_ray = Ray3::new(intersection.position, light_dir);
    }

    // if not clamp, will get white point maybe caused by intersection precision
    Color::new((energy / self.exposure_upper_bound).min(Vec3::splat(1.0)))
  }
}
