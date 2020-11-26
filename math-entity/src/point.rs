use rendiation_math::Vector;

use crate::{Positioned, SpaceEntity};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<T>(pub T);

impl<T: Copy> Point<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}

impl<T: Positioned<f32, D>, const D: usize> SpaceEntity<D> for Point<T> {}
impl<T, const D: usize> SpaceEntity<D> for Vector<T, D> {}
