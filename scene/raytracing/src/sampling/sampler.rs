use std::sync::Arc;

use rand::{prelude::SliceRandom, rngs::ThreadRng, Rng};
use rendiation_statistics::*;

pub struct SamplePrecomputedRequest {
  pub min_spp: usize,
  pub max_1d_dimension: usize,
  pub max_2d_dimension: usize,
}

pub trait SampleGenerator: Default {
  fn override_spp(&self, requested_min_spp: usize) -> usize;
  fn gen_1d(&self, index: usize) -> f32;
  fn gen_2d(&self, index: usize) -> (f32, f32);
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

impl SampleStorage {
  pub fn generate<G: SampleGenerator>(request: SamplePrecomputedRequest) -> Self {
    let gen = G::default();
    let spp = gen.override_spp(request.min_spp);

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
