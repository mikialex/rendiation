pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod line_segment;
pub mod triangle;
pub mod point;
pub mod aabb;

pub use line_segment::*;
pub use triangle::*;
pub use point::*;
pub use aabb::*;

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
