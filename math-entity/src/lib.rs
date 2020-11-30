#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(never_type)]
#![feature(specialization)]
#![allow(incomplete_features)]

// specialization impl
pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod beziersegment;
pub mod hyperaabb;
pub mod hyperplane;
pub mod hyperray;
pub mod hypersphere;
pub mod line_segment;
pub mod point;
pub mod triangle;
pub mod wasm;

pub use beziersegment::*;
pub use hyperaabb::*;
pub use hyperplane::*;
pub use hyperray::*;
pub use hypersphere::*;
pub use line_segment::*;
pub use point::*;
use rendiation_math::*;
pub use triangle::*;
pub use wasm::*;

pub trait SpaceAxis<const D: usize>: Copy {}
pub trait Positioned<T: Scalar, const D: usize>: Copy {
  fn position(&self) -> VectorType<T, D>;
  fn position_mut(&mut self) -> &mut VectorType<T, D>;
}

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
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

pub trait AreaMeasurable<T: Scalar>: LebesgueMeasurable<T, 2> {
  #[inline(always)]
  fn area(&self) -> T {
    self.measure()
  }
}
impl<T: Scalar> AreaMeasurable<T> for T where T: LebesgueMeasurable<T, 2> {}

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
}

pub trait ContainAble<T: Scalar, Target: SpaceEntity<T, D>, const D: usize>:
  SolidEntity<T, D>
{
  fn contains(&self, items_to_contain: &Target) -> bool;
}

pub trait SpaceBounding<T: Scalar, Bound: SolidEntity<T, D>, const D: usize>:
  SpaceEntity<T, D>
{
  fn to_bounding(&self) -> Bound;
}

pub trait SpaceLineSegment<T: Scalar, const D: usize> {
  fn start(&self) -> VectorType<T, D>;
  fn end(&self) -> VectorType<T, D>;
  fn sample(&self, t: T) -> VectorType<T, D>;
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
