pub mod dimension;
pub use dimension::*;

pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod aabb;
pub mod line_segment;
pub mod point;
pub mod triangle;
pub mod md_line;
pub mod md_circle;
pub mod ray;

pub use aabb::*;
pub use line_segment::*;
pub use point::*;
pub use triangle::*;
pub use md_line::*;
pub use md_circle::*;
pub use ray::*;

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

// this not work, conflict impl
// impl<T, Target, Result, Parameter> IntersectAble<Target, Result, Parameter> for T
//   where Target: IntersectAble<T, Result, Parameter>
// {
//   fn intersect(&self, other: &Target, param: &Parameter) -> Result{
//     IntersectAble::<T, Result, Parameter>::intersect(other, self, param)
//   }
// }

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
