use rendiation_math::*;

pub trait DimensionSuccessor<T: DimensionOne<T>, NextDimensionType> {
  fn upgrade_dimension(self, unit: T) -> NextDimensionType;
}

pub trait DimensionPredecessor<T: DimensionOne<T>, PreDimensionType> {
  fn downgrade_dimension(self) -> PreDimensionType;
}

pub trait DimensionOne<T>: Copy {}
pub trait DimensionTwo<T>: Copy {}
pub trait DimensionThree<T>: Copy {}

impl<T: Copy> DimensionOne<T> for T {}
impl<T: Copy> DimensionTwo<T> for Vec2<T> {}
impl<T: Copy> DimensionThree<T> for Vec3<T> {}

impl<T: Copy> DimensionSuccessor<T, Vec2<T>> for T {
  fn upgrade_dimension(self, unit: T) -> Vec2<T> {
    (self, unit).into()
  }
}

impl<T: Copy> DimensionPredecessor<T, T> for Vec2<T> {
  fn downgrade_dimension(self) -> T {
    self.x
  }
}

impl<T: Copy> DimensionSuccessor<T, Vec3<T>> for Vec2<T> {
  fn upgrade_dimension(self, unit: T) -> Vec3<T> {
    (self.x, self.y, unit).into()
  }
}

impl<T: Copy> DimensionPredecessor<T, Vec2<T>> for Vec3<T> {
  fn downgrade_dimension(self) -> Vec2<T> {
    (self.x, self.y).into()
  }
}

#[test]
fn test() {
  let a = 1.upgrade_dimension(2).upgrade_dimension(3);
  assert_eq!(a, Vec3::new(1, 2, 3));
  assert_eq!(a.downgrade_dimension(), Vec2::new(1, 2));
  assert_eq!(a.downgrade_dimension().downgrade_dimension(), 1);
}
