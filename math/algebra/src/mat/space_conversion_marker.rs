use std::{marker::PhantomData, ops::Mul};

use crate::{Scalar, SquareMatrix};

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct SpaceConversionMatrix<M, From, To> {
  value: M,
  from_space: PhantomData<From>,
  to_space: PhantomData<To>,
}

fn space_conversion<M, From, To>(value: M) -> SpaceConversionMatrix<M, From, To> {
  SpaceConversionMatrix {
    value,
    from_space: PhantomData,
    to_space: PhantomData,
  }
}

impl<M, From, To, Next> Mul<SpaceConversionMatrix<M, To, Next>>
  for SpaceConversionMatrix<M, From, To>
where
  M: Mul<M, Output = M>,
{
  type Output = SpaceConversionMatrix<M, From, Next>;

  fn mul(self, m: SpaceConversionMatrix<M, To, Next>) -> Self::Output {
    space_conversion(self.value * m.value)
  }
}

impl<M, From, To> SpaceConversionMatrix<M, From, To> {
  pub fn inverse<T>(&self) -> Option<SpaceConversionMatrix<M, To, From>>
  where
    T: Scalar,
    M: SquareMatrix<T>,
  {
    self.value.inverse().map(space_conversion)
  }

  pub fn inverse_or_identity<T>(&self) -> SpaceConversionMatrix<M, To, From>
  where
    T: Scalar,
    M: SquareMatrix<T>,
  {
    space_conversion(self.value.inverse().unwrap_or(M::one()))
  }
}

pub struct ScreenSpace;
pub struct ClipSpace;
pub struct CameraSpace;
pub struct WorldSpace;
pub struct ObjectSpace;

#[test]
fn test() {
  use crate::*;
  let world_matrix = space_conversion::<Mat4<f32>, ObjectSpace, WorldSpace>(Mat4::one());
  let view_matrix = space_conversion::<Mat4<f32>, WorldSpace, CameraSpace>(Mat4::one());
  let projection_matrix = space_conversion::<Mat4<f32>, CameraSpace, ClipSpace>(Mat4::one());
  let _mvp: SpaceConversionMatrix<Mat4<f32>, ObjectSpace, ClipSpace> =
    world_matrix * view_matrix * projection_matrix;
}
