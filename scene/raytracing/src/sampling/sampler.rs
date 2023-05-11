use std::sync::Arc;

use rand::{prelude::SliceRandom, rngs::ThreadRng, Rng};
use rendiation_algebra::Vec2;

/// https://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface#fragment-SamplerInterface-2

/// Because sample values must be strictly less than 1,
/// OneMinusEpsilon, that represents the largest representable floating-point constant that is less than 1.
/// Later, we will clamp sample vector values to be no larger than this value.
// const ONE_MINUS_EPSILON: f32 = 0x1.ffffffep - 1;

/// The task of a Sampler is to generate a sequence of -dimensional samples in
/// [0, 1) ^ d
pub trait Sampler {
  fn reset(&mut self, next_sampling_index: usize);

  fn next(&mut self) -> f32;

  /// While a 2D sample value could be constructed by using values returned by a pair of calls to sample(),
  /// some samplers can generate better point distributions if they know that two dimensions will be used together.
  fn next_2d(&mut self) -> (f32, f32);
  fn next_2d_vec(&mut self) -> Vec2<f32> {
    Vec2::from(self.next_2d())
  }
}

#[derive(Clone)]
pub struct SampleStorage {
  samples_1d_arrays: Vec<Vec<f32>>,
  samples_2d_arrays: Vec<Vec<(f32, f32)>>,
}

impl SampleStorage {
  pub fn shuffle(&mut self) {
    let mut rng = ThreadRng::default();
    self.samples_1d_arrays.iter_mut().for_each(|v| {
      v.as_mut_slice().shuffle(&mut rng);
    });
    self.samples_2d_arrays.iter_mut().for_each(|v| {
      v.as_mut_slice().shuffle(&mut rng);
    });
  }
}

pub struct SamplePrecomputedRequest {
  pub min_spp: usize,
  pub max_1d_dimension: usize,
  pub max_2d_dimension: usize,
}

pub trait SampleGenerator: Default {
  fn override_ssp(&self, ssp: usize) -> usize;
  fn gen_1d(&self, index: usize) -> f32;
  fn gen_2d(&self, index: usize) -> (f32, f32);
}

impl SampleStorage {
  pub fn generate<G: SampleGenerator>(request: SamplePrecomputedRequest) -> Self {
    let gen = G::default();
    let spp = gen.override_ssp(request.min_spp);

    let mut samples_1d_arrays: Vec<Vec<f32>> = (0..request.max_1d_dimension)
      .map(|_| (0..spp).map(|i| gen.gen_1d(i)).collect())
      .collect();

    let mut samples_2d_arrays: Vec<Vec<(f32, f32)>> = (0..request.max_2d_dimension)
      .map(|_| (0..spp).map(|i| gen.gen_2d(i)).collect())
      .collect();

    let mut rng = ThreadRng::default();
    samples_1d_arrays.iter_mut().for_each(|v| {
      v.as_mut_slice().shuffle(&mut rng);
    });
    samples_2d_arrays.iter_mut().for_each(|v| {
      v.as_mut_slice().shuffle(&mut rng);
    });

    Self {
      samples_1d_arrays,
      samples_2d_arrays,
    }
  }
}

pub struct SamplingStorageState {
  current_sampling_index: usize,
  current_1d_index: usize,
  current_2d_index: usize,
}

pub struct PrecomputedSampler {
  storage: Arc<SampleStorage>,
  state: SamplingStorageState,
  backup: RngSampler,
}

impl PrecomputedSampler {
  pub fn new(source: &Arc<SampleStorage>) -> Self {
    Self {
      storage: source.clone(),
      state: SamplingStorageState {
        current_sampling_index: 0,
        current_1d_index: 0,
        current_2d_index: 0,
      },
      backup: Default::default(),
    }
  }
}

impl Sampler for PrecomputedSampler {
  fn reset(&mut self, next_sampling_index: usize) {
    self.state.current_1d_index = 0;
    self.state.current_2d_index = 0;
    self.state.current_sampling_index = next_sampling_index;
  }

  fn next(&mut self) -> f32 {
    if let Some(array) = self
      .storage
      .samples_1d_arrays
      .get(self.state.current_1d_index)
    {
      self.state.current_1d_index += 1;
      if let Some(sample) = array.get(self.state.current_sampling_index) {
        *sample
      } else {
        self.backup.next()
      }
    } else {
      self.backup.next()
    }
  }

  fn next_2d(&mut self) -> (f32, f32) {
    if let Some(array) = self
      .storage
      .samples_2d_arrays
      .get(self.state.current_2d_index)
    {
      self.state.current_2d_index += 1;
      if let Some(sample) = array.get(self.state.current_sampling_index) {
        *sample
      } else {
        self.backup.next_2d()
      }
    } else {
      self.backup.next_2d()
    }
  }
}

#[derive(Default)]
pub struct RngSampler {
  rng: ThreadRng,
}

impl Sampler for RngSampler {
  fn next(&mut self) -> f32 {
    self.rng.gen()
  }

  fn next_2d(&mut self) -> (f32, f32) {
    (self.rng.gen(), self.rng.gen())
  }

  fn reset(&mut self, _next_sampling_index: usize) {}
}
