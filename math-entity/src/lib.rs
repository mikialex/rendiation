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

pub mod hyperaabb;
pub mod hyperplane;
pub mod hyperray;
pub mod hypersphere;
pub mod line_segment;
pub mod point;
pub mod triangle;
pub mod wasm;

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
}

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

pub trait SpaceEntity<const D: usize> {}

/// https://en.wikipedia.org/wiki/Lebesgue_measure
pub trait LebesgueMeasurable<const D: usize> {
  type MeasureType;
  fn measure(&self) -> Self::MeasureType;
}
pub trait LengthMeasurable: LebesgueMeasurable<1> {
  #[inline(always)]
  fn length(&self) -> Self::MeasureType {
    self.measure()
  }
}
impl<T> LengthMeasurable for T where T: LebesgueMeasurable<1> {}

pub trait AreaMeasurable: LebesgueMeasurable<2> {
  #[inline(always)]
  fn area(&self) -> Self::MeasureType {
    self.measure()
  }
}
impl<T> AreaMeasurable for T where T: LebesgueMeasurable<2> {}

pub trait VolumeMeasurable: LebesgueMeasurable<3> {
  #[inline(always)]
  fn volume(&self) -> Self::MeasureType {
    self.measure()
  }
}
impl<T> VolumeMeasurable for T where T: LebesgueMeasurable<3> {}

pub trait SurfaceAreaMeasure: SpaceEntity<3> + LebesgueMeasurable<2> {
  #[inline(always)]
  fn surface_area(&self) -> Self::MeasureType {
    self.measure()
  }
}
impl<T> SurfaceAreaMeasure for T where T: SpaceEntity<3> + LebesgueMeasurable<2> {}

pub trait PerimeterMeasure: SpaceEntity<2> + LebesgueMeasurable<1> {
  #[inline(always)]
  fn perimeter(&self) -> Self::MeasureType {
    self.measure()
  }
}
impl<T> PerimeterMeasure for T where T: SpaceEntity<2> + LebesgueMeasurable<1> {}

impl<const D: usize, V: VectorDimension<D>> SpaceEntity<D> for V {}

pub trait SolidEntity<const D: usize>: SpaceEntity<D> + LebesgueMeasurable<D> {}

pub trait ContainAble<Target: SpaceEntity<D>, const D: usize>: SolidEntity<D> {
  fn contains(&self, items_to_contain: &Target) -> bool;
}

pub trait SpaceBounding<Bound: SolidEntity<D>, const D: usize>: SpaceEntity<D> {
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
