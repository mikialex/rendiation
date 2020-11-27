use rendiation_math::*;

use crate::{Positioned, SpaceEntity};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point<T>(pub T);

impl<T: Copy> Point<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}

impl<T: Positioned<f32, D>, const D: usize> SpaceEntity<D> for Point<T> {}
impl<T> SpaceEntity<2> for Vec2<T> {}
impl<T> SpaceEntity<3> for Vec3<T> {}
