use std::sync::Arc;

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

fn get_samples_2d(samples: &mut [(f32, f32)]) {
  let scramble = (
    rand::thread_rng().gen_range(0, u32::MAX),
    rand::thread_rng().gen_range(0, u32::MAX),
  );
  sample_2d(samples, scramble, 0)
}
fn get_samples_1d(samples: &mut [f32]) {
  let scramble = rand::thread_rng().gen_range(0, u32::MAX);
  sample_1d(samples, scramble, 0);
}

/// Generate a 2D pattern of low discrepancy samples to fill the slice
/// sample values will be normalized between [0, 1]
pub fn sample_2d(samples: &mut [(f32, f32)], scramble: (u32, u32), offset: u32) {
  for s in samples.iter_mut().enumerate() {
    *s.1 = sample_02(s.0 as u32 + offset, scramble);
  }
}
/// Generate a 1D pattern of low discrepancy samples to fill the slice
/// sample values will be normalized between [0, 1]
pub fn sample_1d(samples: &mut [f32], scramble: u32, offset: u32) {
  for s in samples.iter_mut().enumerate() {
    *s.1 = van_der_corput(s.0 as u32 + offset, scramble);
  }
}
/// Generate a sample from a scrambled (0, 2) sequence
pub fn sample_02(n: u32, scramble: (u32, u32)) -> (f32, f32) {
  (van_der_corput(n, scramble.0), sobol(n, scramble.1))
}
/// Generate a scrambled Van der Corput sequence value
/// as described by Kollig & Keller (2002) and in PBR
/// method is specialized for base 2
pub fn van_der_corput(mut n: u32, scramble: u32) -> f32 {
  n = (n << 16) | (n >> 16);
  n = ((n & 0x00ff00ff) << 8) | ((n & 0xff00ff00) >> 8);
  n = ((n & 0x0f0f0f0f) << 4) | ((n & 0xf0f0f0f0) >> 4);
  n = ((n & 0x33333333) << 2) | ((n & 0xcccccccc) >> 2);
  n = ((n & 0x55555555) << 1) | ((n & 0xaaaaaaaa) >> 1);
  n ^= scramble;
  f32::min(
    ((n >> 8) & 0xffffff) as f32 / ((1 << 24) as f32),
    1.0 - f32::EPSILON,
  )
}
/// Generate a scrambled Sobol' sequence value
/// as described by Kollig & Keller (2002) and in PBR
/// method is specialized for base 2
pub fn sobol(mut n: u32, mut scramble: u32) -> f32 {
  let mut i = 1 << 31;
  while n != 0 {
    if n & 0x1 != 0 {
      scramble ^= i;
    }
    n >>= 1;
    i ^= i >> 1;
  }
  f32::min(
    ((scramble >> 8) & 0xffffff) as f32 / ((1 << 24) as f32),
    1.0 - f32::EPSILON,
  )
}

impl SampleStorage {
  pub fn generate(request: SamplePrecomputedRequest) -> Self {
    let spp = request.min_spp.next_power_of_two();

    let mut rng = ThreadRng::default();
    let samples_1d_arrays = (0..request.max_1d_dimension)
      // .map(|_| seq1_array.clone())
      .map(|_| {
        let mut v = vec![0.; spp];
        get_samples_1d(&mut v);
        v
      })
      .map(|mut v| {
        v.as_mut_slice().shuffle(&mut rng);
        v
      })
      .collect();

    let samples_2d_arrays = (0..request.max_2d_dimension)
      // .map(|_| seq2_array.clone())
      .map(|_| {
        let mut v = vec![(0., 0.); spp];
        get_samples_2d(&mut v);
        v
      })
      .map(|mut v| {
        v.as_mut_slice().shuffle(&mut rng);
        v
      })
      .collect();

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

use rand::{prelude::SliceRandom, rngs::ThreadRng, Rng};

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

macro_rules! AssertLeType {
  ($left:expr, $right:expr) => {
    [(); $right - $left]
  };
}

macro_rules! AssertEqType {
  ($left:expr, $right: expr) => {
    (AssertLeType!($left, $right), AssertLeType!($right, $left))
  };
}

/// https://github.com/rust-lang/rust/issues/76560
/// https://hackmd.io/OZG_XiLFRs2Xmw5s39jRzA?view
pub struct ConstSampler<const N: usize> {}

impl<const N: usize> ConstSampler<N> {
  pub fn sample<const R: usize>(self) -> ConstSampler<R>
  where
    AssertEqType!(N + 1, R): Sized,
  {
    ConstSampler {}
  }
}

#[cfg(test)]
pub fn test(sampler: ConstSampler<1>) -> ConstSampler<3> {
  let sampler2 = sampler.sample::<2>();
  sampler2.sample::<3>()
}
