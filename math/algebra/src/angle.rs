use crate::Scalar;

/// A simple value wrapper that indicate the inner value type is in degree unit.
/// Avoid possible miss conversion between degree and rad.
///
/// We do not impl any function on it, especially trigonometric function which is only
/// meaningful for rad unit, and we consider the common scalar angle unit type is rad.
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Deg<T> {
  pub value: T,
}

impl<T: Scalar> Deg<T> {
  pub fn by(value: T) -> Self {
    Deg { value }
  }
  pub fn to_rad(&self) -> T {
    self.value * T::pi_by_c180()
  }
  pub fn from_rad(rad: T) -> Self {
    Self::by(rad * T::c180_by_pi())
  }
}
