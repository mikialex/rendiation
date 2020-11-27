#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(never_type)]
#![feature(specialization)]
#![allow(incomplete_features)]

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

pub trait Positioned<T, const D: usize>: Copy {
  fn position(&self) -> <VectorMark<T> as DimensionalVec<T, D>>::Type;
}

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

pub trait SpaceEntity<const D: usize> {}
pub trait SolidEntity<const D: usize>: SpaceEntity<D> {}

pub trait ContainAble<Target: SpaceEntity<D>, const D: usize>: SolidEntity<D> {
  fn contains(&self, items_to_contain: &Target) -> bool;
}

pub trait SpaceBounding<Bound: SolidEntity<D>, const D: usize>: SpaceEntity<D> {
  fn to_bounding(&self) -> Bound;
}

pub trait CurveSegment<T, const D: usize> {
  fn start(&self) -> <VectorMark<T> as DimensionalVec<T, D>>::Type;
  fn end(&self) -> <VectorMark<T> as DimensionalVec<T, D>>::Type;
  fn sample(&self, t: f32) -> <VectorMark<T> as DimensionalVec<T, D>>::Type;
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
