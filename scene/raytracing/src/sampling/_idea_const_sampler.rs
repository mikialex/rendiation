#![allow(dead_code)]

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

pub fn test(sampler: ConstSampler<1>) -> ConstSampler<3> {
  let sampler2 = sampler.sample::<2>();
  sampler2.sample::<3>()
}
