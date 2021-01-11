use std::marker::PhantomData;

use crate::{Scalar, SquareMatrix};

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct SpaceConversionMatrix<M, From, To> {
  value: M,
  from_space: PhantomData<From>,
  to_space: PhantomData<To>,
}

pub fn space_conversion<M, From, To>(value: M) -> SpaceConversionMatrix<M, From, To> {
  SpaceConversionMatrix {
    value,
    from_space: PhantomData,
    to_space: PhantomData,
  }
}

impl<M, From, To> SpaceConversionMatrix<M, From, To> {
  pub fn inverse<T>(&self) -> Option<SpaceConversionMatrix<M, To, From>>
  where
    T: Scalar,
    M: SquareMatrix<T>,
  {
    self.value.inverse().map(|m| space_conversion(m))
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
}
