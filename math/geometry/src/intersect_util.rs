use std::ops::{Deref, DerefMut};

use rendiation_algebra::{Scalar, Vec3, VectorSpace};

use crate::HyperRay;

pub trait HitDistanceCompareAble {
  fn is_near_than(&self, other: &Self) -> bool;
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HitPoint<T: Scalar, V> {
  pub position: V,
  pub distance: T,
}

impl<T: Scalar, V> HitDistanceCompareAble for HitPoint<T, V> {
  fn is_near_than(&self, other: &Self) -> bool {
    self.distance < other.distance
  }
}

impl<T: Scalar, V> HitPoint<T, V> {
  pub fn new(position: V, distance: T) -> Self {
    Self { position, distance }
  }
}

impl<T: Scalar, V: VectorSpace<T>> HyperRay<T, V> {
  pub fn at_into(&self, distance: T) -> HitPoint<T, V> {
    HitPoint::new(self.at(distance), distance)
  }
}

pub type HitPoint3D<T = f32> = HitPoint<T, Vec3<T>>;

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

  #[inline(always)]
  pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Nearest<U> {
    Nearest(self.0.map(f))
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
pub struct HitList<T: Scalar, V>(pub Vec<HitPoint<T, V>>);

pub type HitList3D<T = f32> = HitList<T, Vec3<T>>;

impl<T: Scalar, V> HitList<T, V> {
  pub fn new() -> Self {
    Self(Vec::new())
  }
  pub fn new_with_capacity(size: usize) -> Self {
    Self(Vec::with_capacity(size))
  }
  pub fn push_nearest(&mut self, hit: Nearest<HitPoint<T, V>>) {
    if let Nearest(Some(hit)) = hit {
      self.0.push(hit);
    }
  }
}
