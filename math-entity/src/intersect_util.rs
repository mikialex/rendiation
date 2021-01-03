use std::ops::{Deref, DerefMut};

use rendiation_math::Vec3;

use crate::Ray3;

pub trait HitDistanceCompareAble {
  fn is_near_than(&self, other: &Self) -> bool;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct HitPoint3D {
  pub position: Vec3<f32>,
  pub distance: f32,
}

impl HitDistanceCompareAble for HitPoint3D {
  fn is_near_than(&self, other: &Self) -> bool {
    self.distance < other.distance
  }
}

impl HitPoint3D {
  pub fn new(position: Vec3<f32>, distance: f32) -> Self {
    Self { position, distance }
  }
}

impl Ray3 {
  pub fn at_into(&self, distance: f32) -> HitPoint3D {
    HitPoint3D::new(self.at(distance), distance)
  }
}
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
pub struct IntersectionList3D(pub Vec<HitPoint3D>);

impl IntersectionList3D {
  pub fn new() -> Self {
    Self(Vec::new())
  }
  pub fn new_with_capacity(size: usize) -> Self {
    Self(Vec::with_capacity(size))
  }
  pub fn push_nearest(&mut self, hit: Nearest<HitPoint3D>) {
    if let Nearest(Some(hit)) = hit {
      self.0.push(hit);
    }
  }
}
