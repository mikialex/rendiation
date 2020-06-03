pub mod dimension3;
pub use dimension3::*;
pub mod dimension2;
pub use dimension2::*;

pub mod aabb;
pub mod line_segment;
pub mod point;
pub mod triangle;

pub use aabb::*;
pub use line_segment::*;
pub use point::*;
pub use triangle::*;

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

use rendiation_math::*;

pub trait DimensionSuccessor<T: DimensionOne<T>> {
  type NextDimensionType;
  fn upgrade_dimension(self, unit: T) -> Self::NextDimensionType;
}

pub trait DimensionPredecessor<T: DimensionOne<T>>{
  type PreDimensionType;
  fn downgrade_dimension(self) -> Self::PreDimensionType;
}

pub trait DimensionOne<T>: Copy {}
pub trait DimensionTwo<T>: Copy {}
// pub type DimensionTwo<T> = dyn DimensionPredecessor<T, PreDimensionType=T>;
// pub type DimensionThree<T> = dyn DimensionPredecessor<T, PreDimensionType=DimensionTwo<T>>;

impl<T: Copy> DimensionOne<T> for T {}

impl<T: DimensionOne<T>> DimensionSuccessor<T> for T {
  type NextDimensionType = Vec2<T>;
  fn upgrade_dimension(self, unit: T) -> Self::NextDimensionType {
    (self, unit).into()
  }
}

impl<T: DimensionOne<T>> DimensionPredecessor<T> for Vec2<T> {
  type PreDimensionType = T;
  fn downgrade_dimension(self) -> Self::PreDimensionType {
    self.x
  }
}

impl<T: DimensionOne<T>> DimensionSuccessor<T> for Vec2<T> {
  type NextDimensionType = Vec3<T>;
  fn upgrade_dimension(self, unit: T) -> Self::NextDimensionType {
    (self.x, self.y, unit).into()
  }
}

impl<T: DimensionOne<T>> DimensionPredecessor<T> for Vec3<T> {
  type PreDimensionType = Vec2<T>;
  fn downgrade_dimension(self) -> Self::PreDimensionType {
    (self.x, self.y).into()
  }
}

#[test]
fn test(){
  let a = 1.upgrade_dimension(2).upgrade_dimension(3);
  assert_eq!(a, Vec3::new(1, 2, 3));
  assert_eq!(a.downgrade_dimension(), Vec2::new(1, 2));
  assert_eq!(a.downgrade_dimension().downgrade_dimension(), 1);
}