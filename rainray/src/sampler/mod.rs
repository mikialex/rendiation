use rand::{rngs::StdRng, Rng};
use rendiation_math::Vec2;

pub struct Sampler {
  pub rand: StdRng,
}

// /// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface.html#fig:sampler-comparison
// pub trait Sampler {
//   fn get() -> f32;
//   fn get_2d() -> Vec2<f32>;

//   /// Most Samplers can do a better job of generating some particular sizes of these arrays than others.
//   /// Code that needs arrays of samples should call this method with the desired number of samples to be taken,
//   /// giving the Sampler an opportunity to adjust the number of samples to a better number.
//   ///
//   /// The default implementation returns the given count unchanged.
//   fn round_count(size: usize) -> usize {
//     size
//   }
// }

// struct SampleResult<T> {
//   samples: Vec<T>,
// }

// // /// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Stratified_Sampling.html
// // struct StratifiedSampler {

// // }
