#![feature(const_generics)]
#![feature(const_evaluatable_checked)]
#![feature(never_type)]
#![allow(incomplete_features)]

pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod aabb;
pub mod hyperplane;
pub mod hypersphere;
pub mod line_segment;
pub mod point;
pub mod ray;
pub mod triangle;
pub mod wasm;

pub use aabb::*;
pub use hyperplane::*;
pub use hypersphere::*;
pub use line_segment::*;
pub use point::*;
pub use ray::*;
use rendiation_math::Vector;
pub use triangle::*;
pub use wasm::*;

pub mod transformation;
pub use transformation::*;

pub trait Positioned<T, const D: usize>: Copy {
  fn position(&self) -> Vector<T, D>;
}

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

pub trait ContainAble<Target, const D: usize> {
  fn contains(&self, items_to_contain: &Target) -> bool;
}

pub trait SpaceBounding<Bound> {
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

pub trait CurveSegment<T> {
  fn start(&self) -> T;
  fn end(&self) -> T;

  fn sample(&self, t: f32) -> T;
}
