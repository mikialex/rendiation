use std::ops::{Deref, DerefMut};

use rendiation_math::{Scalar, VectorType};

use crate::HyperRay;

pub trait HitDistanceCompareAble {
  fn is_near_than(&self, other: &Self) -> bool;
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HitPoint<const D: usize, T: Scalar = f32> {
  pub position: VectorType<T, D>,
  pub distance: T,
}

impl<const D: usize, T: Scalar> HitDistanceCompareAble for HitPoint<D, T> {
  fn is_near_than(&self, other: &Self) -> bool {
    self.distance < other.distance
  }
}

impl<const D: usize, T: Scalar> HitPoint<D, T> {
  pub fn new(position: VectorType<T, D>, distance: T) -> Self {
    Self { position, distance }
  }
}

impl<const D: usize, T: Scalar> HyperRay<T, D> {
  pub fn at_into(&self, distance: T) -> HitPoint<D, T> {
    HitPoint::new(self.at(distance), distance)
  }
}

pub type HitPoint3D<T = f32> = HitPoint<3, T>;

#[repr(transparent)]
#[derive(Default, Copy, Clone, Debug)]
pub struct Nearest<T>(pub Option<T>);
impl<T> Nearest<T>
where
  T: HitDistanceCompareAble,
{
  #[inline(always)]
  pub fn none() -> Self {
    Self(None)
  }

  #[inline(always)]
  pub fn some(v: T) -> Self {
    Self(Some(v))
  }

  #[inline(always)]
  pub fn refresh(&mut self, v: T) -> &mut Self {
    if let Some(stored) = &mut self.0 {
      if v.is_near_than(stored) {
        *stored = v;
      }
    } else {
      self.0 = Some(v)
    }
    self
  }

  #[inline(always)]
  pub fn refresh_nearest(&mut self, v: Self) -> &mut Self {
    if let Some(v) = v.0 {
      self.refresh(v);
    }
    self
  }
}

impl<T> Deref for Nearest<T> {
  type Target = Option<T>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for Nearest<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> From<Option<T>> for Nearest<T> {
  fn from(v: Option<T>) -> Self {
    Self(v)
  }
}

#[derive(Default)]
pub struct HitList<const D: usize, T: Scalar = f32>(pub Vec<HitPoint<D, T>>);

pub type HitList3D<T = f32> = HitList<3, T>;

impl<const D: usize, T: Scalar> HitList<D, T> {
  pub fn new() -> Self {
    Self(Vec::new())
  }
  pub fn new_with_capacity(size: usize) -> Self {
    Self(Vec::with_capacity(size))
  }
  pub fn push_nearest(&mut self, hit: Nearest<HitPoint<D, T>>) {
    if let Nearest(Some(hit)) = hit {
      self.0.push(hit);
    }
  }
}
