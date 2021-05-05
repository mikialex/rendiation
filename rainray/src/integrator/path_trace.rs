use rendiation_algebra::{InnerProductSpace, Vector};
use rendiation_color::{Color, LinearRGBColorSpace};
use rendiation_geometry::Ray3;

use super::Integrator;
use crate::{
  math::rand, math::Vec3, BSDFSampleResult, Intersection, LightSampleResult, ModelInstance,
  NormalizedVec3, RayTraceScene,
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
  fn sample_lights<'a>(
    &self,
    scene: &RayTraceScene<'a>,
    model: &ModelInstance<'a>,
    intersection: &Intersection,
    view_dir: NormalizedVec3,
  ) -> Vec3 {
    let mut energy = Vec3::new(0.0, 0.0, 0.0);
    for light in &scene.lights {
      let node = light.node;
      let light = light.light;
      if let Some(LightSampleResult {
        emissive,
        light_in_dir,
      }) = light.sample(intersection.position, scene, node)
      {
        let bsdf = model.bsdf(view_dir, light_in_dir.reverse(), intersection, scene);
        energy += bsdf * emissive * -light_in_dir.dot(intersection.shading_normal);
      }
    }
    energy
  }
}

impl Integrator for PathTraceIntegrator {
  fn integrate<'a>(
    &self,
    scene: &RayTraceScene<'a>,
    ray: Ray3,
  ) -> Color<f32, LinearRGBColorSpace<f32>> {
    let mut energy = Vec3::new(0., 0., 0.);
    let mut throughput = Vec3::new(1., 1., 1.);
    let mut current_ray = ray;

    for _depth in 0..self.bounce_time_limit {
      if let Some((intersection, _, model)) = scene.get_min_dist_hit(current_ray) {
        let view_dir = current_ray.direction.reverse();

        let BSDFSampleResult {
          light_dir,
          bsdf,
          pdf,
        } = model.sample(view_dir, &intersection, scene);

        if pdf == 0.0 {
          break;
        }

        let cos = light_dir.dot(intersection.shading_normal).abs();
        throughput = throughput * cos * bsdf / pdf;

        energy += self.sample_lights(scene, model, &intersection, view_dir) * throughput;

        // roulette exist
        if throughput.max_channel() < self.roulette_threshold {
          if rand() < self.roulette_factor {
            break;
          }
          throughput /= 1. - self.roulette_factor;
        }

        current_ray = Ray3::new(intersection.position, light_dir);
      } else {
        // hit outside scene, sample background;
        if let Some(background) = &scene.scene.background {
          energy += background.sample(&current_ray) * throughput;
          break;
        }
      }
    }

    // if not clamp, will get white point caused by high variance in brdf sampling
    // https://computergraphics.stackexchange.com/questions/8693/where-do-fireflies-come-from
    Color::new((energy / self.exposure_upper_bound).min(Vec3::splat(1.0)))
  }
}
