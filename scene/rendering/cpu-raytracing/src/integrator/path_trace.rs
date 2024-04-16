use rendiation_algebra::RealVector;
use rendiation_algebra::{InnerProductSpace, Vec3, Vector};
use rendiation_color::LinearRGBColor;
use rendiation_geometry::Ray3;

use crate::*;

pub struct PathTraceIntegrator {
  pub sampling_config: AdaptivePixelSamplerConfig,
  pub exposure_upper_bound: f32,
  pub bounce_time_limit: usize,
  pub roulette: RussianRoulette,
}

impl Default for PathTraceIntegrator {
  fn default() -> Self {
    Self {
      sampling_config: Default::default(),
      exposure_upper_bound: 1.0,
      bounce_time_limit: 20,
      roulette: Default::default(),
    }
  }
}

// impl PathTraceIntegrator {
//   // next event estimation
//   fn sample_lights(
//     &self,
//     _target: &target<RayTracingtarget>,
//     _model: &ModelInstance,
//     _intersection: &Intersection,
//     _view_dir: NormalizedVec3<f32>,
//   ) -> Vec3<f32> {
//     // let mut energy = Vec3::new(0.0, 0.0, 0.0);
//     // for light in &target.lights {
//     //   let node = light.node;
//     //   let light = light.light;
//     //   if let Some(LightSampleResult {
//     //     emissive,
//     //     light_in_dir,
//     //   }) = light.sample(intersection.position, target, node)
//     //   {
//     //     let bsdf = model.bsdf(view_dir, light_in_dir.reverse(), intersection);
//     //     energy += bsdf * emissive * -light_in_dir.dot(intersection.shading_normal);
//     //   }
//     // }
//     // energy
//     Vec3::new(0., 0., 0.)
//   }
// }

pub trait RayTraceContentForPathTracing: RayTraceContentBase {
  fn get_min_dist_hit_with_model(&self, world_ray: Ray3) -> Option<(Intersection, f32, &Model)>;
  fn sample_environment(&self, world_ray: Ray3) -> Vec3<f32>;
}

impl<T> Integrator<T> for PathTraceIntegrator
where
  T: RayTraceContentForPathTracing,
{
  type PixelSampler = AdaptivePixelSampler;
  fn create_pixel_sampler(&self) -> Self::PixelSampler {
    self.sampling_config.into()
  }

  type Sampler = PrecomputedSampler;
  // todo optimize, use shuffle iter?
  fn create_sampler(&self) -> Self::Sampler {
    let sampling_cache =
      SampleStorage::generate::<SobolSamplingGenerator>(SamplePrecomputedRequest {
        min_spp: 128,
        max_1d_dimension: 50,
        max_2d_dimension: 50,
      });
    let sampling_cache = std::sync::Arc::new(sampling_cache);
    PrecomputedSampler::new(&sampling_cache)
  }

  fn integrate(&self, target: &T, ray: Ray3, sampler: &mut dyn Sampler) -> LinearRGBColor<f32> {
    let mut energy = Vec3::new(0., 0., 0.);
    let mut throughput = Vec3::new(1., 1., 1.);
    let mut current_ray = ray;

    for _depth in 0..self.bounce_time_limit {
      if let Some((intersection, _, model)) = target.get_min_dist_hit_with_model(current_ray) {
        let view_dir = current_ray.direction.reverse();

        let BRDFImportantSampled {
          sample: light_dir,
          pdf,
          importance: bsdf,
        } = model
          .material
          .sample_light_dir_use_bsdf_importance(view_dir, &intersection, sampler);

        if pdf == 0.0 {
          break;
        }

        let cos = light_dir.dot(intersection.shading_normal).abs();
        throughput = throughput * cos * bsdf / pdf;

        // energy += self.sample_lights(target, model, &intersection, view_dir) * throughput;

        if self.roulette.roulette_exit(&mut throughput) {
          break;
        }

        current_ray = Ray3::new(intersection.position, light_dir);
      } else {
        // hit outside target, sample background;
        energy += target.sample_environment(current_ray) * throughput;

        break;
      }
    }

    // if not clamp, will get white point caused by high variance in brdf sampling
    // https://computergraphics.stackexchange.com/questions/8693/where-do-fireflies-come-from
    (energy / self.exposure_upper_bound)
      .min(Vec3::splat(1.0))
      .into()
  }
}

/// https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/Russian_Roulette_and_Splitting
pub struct RussianRoulette {
  pub roulette_threshold: f32,
  pub roulette_factor: f32,
}

impl Default for RussianRoulette {
  fn default() -> Self {
    Self {
      roulette_threshold: 0.05,
      roulette_factor: 0.05,
    }
  }
}

impl RussianRoulette {
  /// Roulette exit, a classical way to terminate low contribute path in an unbiased way
  ///
  /// return should break sampling
  pub fn roulette_exit(&self, throughput: &mut Vec3<f32>) -> bool {
    if throughput.max_channel() < self.roulette_threshold {
      if rand::random::<f32>() < self.roulette_factor {
        return true;
      }
      *throughput /= 1. - self.roulette_factor;
    }
    false
  }
}
