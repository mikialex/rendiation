use crate::*;

/// A simple value wrapper that indicate the inner value type is in degree unit.
/// Avoid possible miss conversion between degree and rad.
///
/// We do not impl any function on it, especially trigonometric function which is only
/// meaningful for rad unit, and we consider the common scalar angle unit type is rad.
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Deg<T> {
  pub value: T,
}

impl<T: Scalar> Deg<T> {
  /// Wrap a value in degree unit.
  pub fn by(value: T) -> Self {
    Deg { value }
  }
  /// Convert into radians.
  pub fn to_rad(&self) -> T {
    self.value * T::pi_by_c180()
  }
  /// Create from radians.
  pub fn from_rad(rad: T) -> Self {
    Self::by(rad * T::c180_by_pi())
  }
}

#[test]
fn deg_rad_conversion() {
  let d = Deg::by(180.0_f64);
  assert!((d.to_rad() - std::f64::consts::PI).abs() < 1e-6);
  let d = Deg::<f64>::from_rad(std::f64::consts::FRAC_PI_2);
  assert!((d.value - 90.0).abs() < 1e-4);
  let d = Deg::by(37.0_f32);
  assert!((Deg::from_rad(d.to_rad()).value - 37.0).abs() < 1e-4);
}
