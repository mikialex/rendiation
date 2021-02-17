use rendiation_algebra::{Scalar, Vec3};

use crate::Texture2D;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CubeTextureFace {
  PositiveX = 0,
  NegativeX = 1,
  PositiveY = 2,
  NegativeY = 3,
  PositiveZ = 4,
  NegativeZ = 5,
}

// https://github.com/Hyper3D/hyper3d-envmapgen/blob/master/rust/src/cubemap.rs

pub struct CubeTexture<T> {
  pub positive_x: T,
  pub negative_x: T,

  pub positive_y: T,
  pub negative_y: T,

  pub positive_z: T,
  pub negative_z: T,
}

// impl<T> CubeTexture<T> {
//   pub fn get_face(&self, face: CubeTextureFace) -> &T{
//     use CubeTextureFace::*;
//     match face{

//     }
//   }
// }

// http://www.cim.mcgill.ca/~langer/557/18-slides.pdf
impl<P, T: Texture2D<Pixel = P>> CubeTexture<T> {
  pub fn sample<S: Scalar>(&self, _direction: Vec3<S>) -> P {
    todo!()
  }
}
