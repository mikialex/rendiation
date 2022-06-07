use rendiation_algebra::*;

/// https://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface#fragment-SamplerInterface-2

/// Because sample values must be strictly less than 1,
/// OneMinusEpsilon, that represents the largest representable floating-point constant that is less than 1.
/// Later, we will clamp sample vector values to be no larger than this value.
// const ONE_MINUS_EPSILON: f32 = 0x1.ffffffep - 1;

/// The task of a Sampler is to generate a sequence of -dimensional samples in
/// [0, 1) ^ d
pub trait Sampler {
  fn next(&mut self) -> f32;

  /// While a 2D sample value could be constructed by using values returned by a pair of calls to sample(),
  /// some samplers can generate better point distributions if they know that two dimensions will be used together.
  fn next_2d(&mut self) -> Vec2<f32>;

  /// For convenience, the Sampler base class provides a method that initializes a CameraSample for a given pixel.
  fn next_camera(&mut self, pixel: Vec2<usize>) -> CameraSample {
    CameraSample {
      film_position: pixel.map(|v| v as f32) + self.next_2d(),
      time: self.next(),
      lens: self.next_2d(),
    }
  }

  /// When the rendering algorithm is ready to start work on a given pixel,
  /// it starts by calling StartPixel(), providing the coordinates of the pixel in the image.
  ///
  /// Some Sampler implementations use the knowledge of which pixel is being sampled to improve the
  /// overall distribution of the samples that they generate for the pixel, while others ignore this information.
  fn start_pixel(&mut self, _pixel: Vec2<usize>) {}
  fn next_samples(&mut self) {}
}

pub struct CameraSample {
  pub film_position: Vec2<f32>,
  pub time: f32,
  pub lens: Vec2<f32>,
}

pub trait PixelSampler: Sized {
  fn should_sample(&self) -> bool;
  fn update_sample_result(&mut self, result: Vec3<f32>);
  fn take_result(self) -> Vec3<f32>;

  fn sample_pixel(mut self, per_sample: impl Fn() -> Vec3<f32>) -> Vec3<f32> {
    loop {
      if !self.should_sample() {
        break self.take_result();
      }
      self.update_sample_result(per_sample())
    }
  }
}

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

impl PixelSampler for FixedSamplesPerPixel {
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
}

// /// Each storage contains samples need by one time pixel sampling
// pub struct SampleStorage {
//   samples_1d_array: Vec<f32>,
//   samples_2d_array: Vec<Vec2<f32>>,
// }

// pub struct PixelSamplerDispatcher<T> {
//   sampler: T,
//   pixel_in_sampling: Vec2<usize>,
//   current_sample_count: usize,
//   sample_per_pixel: usize,
//   pre_computed_samples: Vec<SampleStorage>,
// }

use rand::{rngs::ThreadRng, Rng};

#[derive(Default)]
pub struct RngSampler {
  rng: ThreadRng,
}

impl Sampler for RngSampler {
  fn next(&mut self) -> f32 {
    self.rng.gen()
  }

  fn next_2d(&mut self) -> Vec2<f32> {
    Vec2::new(self.rng.gen(), self.rng.gen())
  }
}
