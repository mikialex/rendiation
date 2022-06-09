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
  fn next(&mut self) -> f32;

  /// While a 2D sample value could be constructed by using values returned by a pair of calls to sample(),
  /// some samplers can generate better point distributions if they know that two dimensions will be used together.
  fn next_2d(&mut self) -> (f32, f32);
  fn next_2d_vec(&mut self) -> Vec2<f32> {
    Vec2::from(self.next_2d())
  }
}

/// Contains samples need by one sequence of multidimensional pixel sampling
pub struct OneSampleStorage {
  samples_1d_array: Vec<f32>,
  samples_2d_array: Vec<(f32, f32)>,
}

pub struct SampleStorage {
  /// storage for each sampling
  samples: Vec<OneSampleStorage>,
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

impl Sampler for PrecomputedSampler {
  fn next(&mut self) -> f32 {
    if let Some(one_storage) = self.storage.samples.get(self.state.current_sampling_index) {
      if let Some(sample) = one_storage
        .samples_1d_array
        .get(self.state.current_1d_index)
      {
        self.state.current_1d_index += 1;
        *sample
      } else {
        self.backup.next()
      }
    } else {
      self.backup.next()
    }
  }

  fn next_2d(&mut self) -> (f32, f32) {
    if let Some(one_storage) = self.storage.samples.get(self.state.current_sampling_index) {
      if let Some(sample) = one_storage
        .samples_2d_array
        .get(self.state.current_2d_index)
      {
        self.state.current_2d_index += 1;
        *sample
      } else {
        self.backup.next_2d()
      }
    } else {
      self.backup.next_2d()
    }
  }
}

use rand::{rngs::ThreadRng, Rng};

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
}

// use sobol::params::JoeKuoD6;
// use sobol::Sobol;

// pub struct SobolSequence {
//   generator: usize,
// }

// impl Sampler for SobolSequence {
//   fn next(&mut self) -> f32 {
//     let params = JoeKuoD6::minimal();
//     let seq = Sobol::<f32>::new(300, &params);

//     for point in seq.take(100) {
//       println!("{:?}", point);
//     }
//     todo!()
//   }

//   fn next_2d(&mut self) -> (f32, f32) {
//     (self.rng.gen(), self.rng.gen())
//   }
// }

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
