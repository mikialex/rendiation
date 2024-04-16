use rendiation_algebra::*;

pub trait PixelSampleController: Sized {
  /// decide if the current pixel should continue sampling
  fn should_sample(&self) -> bool;
  /// update the internal control state with new sample result
  fn update_sample_result(&mut self, result: Vec3<f32>);
  /// if the pixel sampling is sufficient, take self and output the final result
  fn take_result(self) -> Vec3<f32>;
  /// return the sample index (how many times has sampled for this pixel), the sampler require this
  /// to get correct sample.
  fn next_sample_index(&self) -> usize;

  /// the generalized sample logic for a pixel, pass the per sample logic in.
  fn sample_pixel(mut self, mut per_sample: impl FnMut(usize) -> Vec3<f32>) -> Vec3<f32> {
    loop {
      if !self.should_sample() {
        break self.take_result();
      }
      self.update_sample_result(per_sample(self.next_sample_index()))
    }
  }
}

/// the naive sampler control with fixed sample count for each pixel
pub struct FixedSamplesPerPixel {
  target_samples_per_pixel: usize,
  current_samples: usize,
  accumulate: Vec3<f32>,
}

impl FixedSamplesPerPixel {
  pub fn by_target_samples_per_pixel(target_samples_per_pixel: usize) -> Self {
    Self {
      target_samples_per_pixel,
      current_samples: 0,
      accumulate: Vec3::zero(),
    }
  }
}

impl PixelSampleController for FixedSamplesPerPixel {
  fn should_sample(&self) -> bool {
    self.current_samples < self.target_samples_per_pixel
  }

  fn update_sample_result(&mut self, result: Vec3<f32>) {
    self.current_samples += 1;
    self.accumulate += result;
  }

  fn take_result(self) -> Vec3<f32> {
    if self.current_samples == 0 {
      self.accumulate
    } else {
      self.accumulate / self.current_samples as f32
    }
  }
  fn next_sample_index(&self) -> usize {
    self.current_samples
  }
}

/// http://luthuli.cs.uiuc.edu/~daf/courses/Rendering/Papers-2/RTHWJ.article.pdf
/// https://www.researchgate.net/publication/220721426_Antialiased_ray_tracing_by_adaptive_progressive_refinement
///
/// ## Summarize
///
/// For most of pixels, their samples are assumed to be normal distributed. So we could estimate the
/// expectation(the final result) within a given tolerance range with in a  given confidence
/// interval by using the Student-T distribution.
pub struct AdaptivePixelSampler {
  config: AdaptivePixelSamplerConfig,

  variance: f32,
  current_m2_accumulate: f32,
  current_samples: usize,
  accumulate: Vec3<f32>,
}

impl From<AdaptivePixelSamplerConfig> for AdaptivePixelSampler {
  fn from(config: AdaptivePixelSamplerConfig) -> Self {
    Self {
      config,
      variance: 0.,
      current_m2_accumulate: 0.,
      current_samples: 0,
      accumulate: Vec3::zero(),
    }
  }
}

#[derive(Copy, Clone)]
pub struct AdaptivePixelSamplerConfig {
  min_sample_count: usize,
  max_sample_count: usize,
  /// tolerance upper bound - tolerance lower bound
  tolerance_width: f32,
  confidence_level: f32,
}

impl Default for AdaptivePixelSamplerConfig {
  fn default() -> Self {
    Self {
      min_sample_count: 32,
      max_sample_count: 128,
      tolerance_width: 0.02,
      confidence_level: 0.95,
    }
  }
}

impl PixelSampleController for AdaptivePixelSampler {
  fn should_sample(&self) -> bool {
    let config = &self.config;
    if self.current_samples < config.min_sample_count {
      return true;
    }

    if self.current_samples >= config.max_sample_count {
      return false;
    }

    use statrs::distribution::{ContinuousCDF, StudentsT};
    let student_t = StudentsT::new(0.0, 1.0, (self.current_samples - 1) as f64).unwrap();
    let alpha = 1.0 - config.confidence_level;
    let tolerance_width = 2.
      * student_t.inverse_cdf(1.0 - alpha as f64 / 2.0) as f32
      * (self.variance / self.current_samples as f32).sqrt();

    tolerance_width > config.tolerance_width
  }

  fn update_sample_result(&mut self, result: Vec3<f32>) {
    self.current_samples += 1;
    self.accumulate += result;

    // todo use luminance
    fn result_to_scalar(v: Vec3<f32>) -> f32 {
      (v.x + v.y + v.z) / 3.
    }

    // https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance
    let mean = result_to_scalar(self.accumulate / self.current_samples as f32);
    let new = result_to_scalar(result);
    let delta = new - mean;
    let next_mean = mean + delta / self.current_samples as f32;
    let delta2 = new - next_mean;
    self.current_m2_accumulate += delta * delta2;

    self.variance = self.current_m2_accumulate / (self.current_samples - 1) as f32;
  }

  fn take_result(self) -> Vec3<f32> {
    // return Vec3::splat(
    //   (self.current_samples - self.config.min_sample_count) as f32
    //     / self.config.max_sample_count as f32,
    // );
    if self.current_samples == 0 {
      self.accumulate
    } else {
      self.accumulate / self.current_samples as f32
    }
  }
  fn next_sample_index(&self) -> usize {
    self.current_samples
  }
}
