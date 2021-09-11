#![allow(clippy::suspicious_operation_groupings)]

use rendiation_algebra::*;

pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod beziersegment;
pub mod hyperaabb;
pub mod hyperellipse;
pub mod hyperplane;
pub mod hyperray;
pub mod hypersphere;
pub mod intersect_util;
pub mod line_segment;
pub mod point;
pub mod segment;
pub mod triangle;
pub mod space_line;

pub use beziersegment::*;
pub use hyperaabb::*;
pub use hyperellipse::*;
pub use hyperplane::*;
pub use hyperray::*;
pub use hypersphere::*;
pub use intersect_util::*;
pub use line_segment::*;
pub use point::*;
pub use segment::*;
pub use triangle::*;
pub use space_line::*;

pub trait Positioned {
  type Position;
  fn position(&self) -> &Self::Position;
  fn mut_position(&mut self) -> &mut Self::Position;
}

pub trait SpaceAxis<const D: usize>: Copy {}

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

pub trait DistanceTo<Target, T: Scalar, Parameter = ()> {
  fn distance_to(&self, other: &Target) -> T;
}

pub trait DistanceSquareTo<Target, T: Scalar, Parameter = ()> {
  fn distance_sq_to(&self, other: &Target) -> T;

  fn distance_to(&self, other: &Target) -> T {
    self.distance_sq_to(other).sqrt()
  }
}

/// https://en.wikipedia.org/wiki/Lebesgue_measure
pub trait LebesgueMeasurable<T: Scalar, const D: usize> {
  fn measure(&self) -> T;
}
pub trait LengthMeasurable<T: Scalar>: LebesgueMeasurable<T, 1> {
  #[inline(always)]
  fn length(&self) -> T {
    self.measure()
  }
}
impl<T: Scalar> LengthMeasurable<T> for T where T: LebesgueMeasurable<T, 1> {}

pub trait AreaMeasurable<T: Scalar>: SpaceEntity<T, 2> + LebesgueMeasurable<T, 2> {
  #[inline(always)]
  fn area(&self) -> T {
    self.measure()
  }
}
impl<T: Scalar> AreaMeasurable<T> for T where T: SpaceEntity<T, 2> + LebesgueMeasurable<T, 2> {}

pub trait VolumeMeasurable<T: Scalar>: LebesgueMeasurable<T, 3> {
  #[inline(always)]
  fn volume(&self) -> T {
    self.measure()
  }
}
impl<T: Scalar> VolumeMeasurable<T> for T where T: LebesgueMeasurable<T, 3> {}

pub trait SurfaceAreaMeasure<T: Scalar>: SpaceEntity<T, 3> + LebesgueMeasurable<T, 2> {
  #[inline(always)]
  fn surface_area(&self) -> T {
    self.measure()
  }
}
impl<T: Scalar> SurfaceAreaMeasure<T> for T where T: SpaceEntity<T, 3> + LebesgueMeasurable<T, 2> {}

pub trait PerimeterMeasure<T: Scalar>: SpaceEntity<T, 2> + LebesgueMeasurable<T, 1> {
  #[inline(always)]
  fn perimeter(&self) -> T {
    self.measure()
  }
}
impl<T: Scalar> PerimeterMeasure<T> for T where T: SpaceEntity<T, 2> + LebesgueMeasurable<T, 1> {}

pub trait SolidEntity<T: Scalar, const D: usize>:
  SpaceEntity<T, D> + LebesgueMeasurable<T, D>
{
  type Center;
  fn centroid(&self) -> Self::Center;
}

pub trait ContainAble<T, Target, const D: usize>: SolidEntity<T, D>
where
  T: Scalar,
  Target: SpaceEntity<T, D>,
{
  fn contains(&self, items_to_contain: &Target) -> bool;
}

pub trait SpaceBounding<T, Bound, const D: usize>: SpaceEntity<T, D>
where
  T: Scalar,
  Bound: SolidEntity<T, D>,
{
  fn to_bounding(&self) -> Bound;
}


#[macro_export]
macro_rules! intersect_reverse {
  ($self_item: ty, $result:ty, $param:ty, $target:ty) => {
    impl IntersectAble<$target, $result, $param> for $self_item {
      fn intersect(&self, other: &$target, p: &$param) -> $result {
        IntersectAble::<$self_item, $result, $param>::intersect(other, self, p)
      }
    }
  };
}
