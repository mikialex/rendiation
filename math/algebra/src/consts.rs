pub trait Number<T> {
  fn number<const N: f32>() -> T;
}

impl<T: From<f32>> Number<T> for T {
  fn number<const N: f32>() -> T {
    N.into()
  }
}

#[test]
fn const_eval() {
  assert_eq!(f32::number::<1.5>(), 1.5);
  assert_eq!(f64::number::<1.5>(), 1.5);
}

pub trait Two: Sized {
  #[must_use]
  fn two() -> Self;
}
pub trait Three: Sized {
  #[must_use]
  fn three() -> Self;
}

pub trait Half: Sized {
  #[must_use]
  fn half() -> Self;
}

pub trait PiByC180: Sized {
  #[must_use]
  fn pi_by_c180() -> Self;
}
pub trait C180ByPi: Sized {
  #[must_use]
  fn c180_by_pi() -> Self;
}

impl Two for f32 {
  #[inline(always)]
  fn two() -> Self {
    2.0_f32
  }
}
impl Three for f32 {
  #[inline(always)]
  fn three() -> Self {
    3.0_f32
  }
}
impl Half for f32 {
  #[inline(always)]
  fn half() -> Self {
    0.5_f32
  }
}

impl PiByC180 for f32 {
  #[inline(always)]
  fn pi_by_c180() -> Self {
    std::f32::consts::PI / 180.0
  }
}
impl C180ByPi for f32 {
  #[inline(always)]
  fn c180_by_pi() -> Self {
    180.0 / std::f32::consts::PI
  }
}

impl Two for f64 {
  #[inline(always)]
  fn two() -> Self {
    2.0_f64
  }
}
impl Three for f64 {
  #[inline(always)]
  fn three() -> Self {
    3.0_f64
  }
}
impl Half for f64 {
  #[inline(always)]
  fn half() -> Self {
    0.5_f64
  }
}
impl PiByC180 for f64 {
  #[inline(always)]
  fn pi_by_c180() -> Self {
    std::f64::consts::PI / 180.0
  }
}
impl C180ByPi for f64 {
  #[inline(always)]
  fn c180_by_pi() -> Self {
    180.0 / std::f64::consts::PI
  }
}

impl Two for i32 {
  #[inline(always)]
  fn two() -> Self {
    2
  }
}
impl Three for i32 {
  #[inline(always)]
  fn three() -> Self {
    3
  }
}

impl Two for i64 {
  #[inline(always)]
  fn two() -> Self {
    2
  }
}
impl Three for i64 {
  #[inline(always)]
  fn three() -> Self {
    3
  }
}
