use rendiation_algebra::Scalar;

/// https://en.wikipedia.org/wiki/Sinc_function
pub fn sinc<T: Scalar>(x: T) -> T {
  let a = x * T::PI();

  if x == T::zero() {
    T::one()
  } else {
    a.sin() / a
  }
}

#[test]
fn sinc_eval() {
  assert_eq!(sinc(0.0_f32), 1.);
  assert_eq!(sinc(0.0_f32 + f32::EPSILON), 1.);
  assert!(sinc(1.0_f32) - 0. <= f32::EPSILON);
}

/// https://en.wikipedia.org/wiki/Lanczos_resampling
pub fn lanczos<T: Scalar>(x: T, a: T) -> T {
  if x.abs() > a {
    return T::zero();
  }

  sinc(x) * sinc(x / a)
}

#[test]
fn lanczos_eval() {
  assert_eq!(lanczos(0.0_f32, 1.), 1.);
  assert_eq!(lanczos(0.0_f32 + f32::EPSILON, 1.), 1.);
  assert!(lanczos(1.0_f32, 1.) - 0. <= f32::EPSILON);
  assert!(lanczos(2.0_f32, 2.) - 0. <= f32::EPSILON);
}
