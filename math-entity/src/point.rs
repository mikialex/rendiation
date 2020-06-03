#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point3<T>(pub T);

impl<T: Copy> Point3<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}
