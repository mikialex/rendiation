#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<T>(pub T);

impl<T: Copy> Point<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}
