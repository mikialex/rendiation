use rendiation_algebra::{NormalizedVector, Scalar, Vec2, Vec3};

use crate::{Texture2D, Texture2dSampleAble, TextureFilterMode, TextureSampler};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CubeTextureFace {
  PositiveX,
  NegativeX,
  PositiveY,
  NegativeY,
  PositiveZ,
  NegativeZ,
}

// https://github.com/Hyper3D/hyper3d-envmapgen/blob/master/rust/src/cubemap.rs

pub struct CubeTexture<P, T>
where
  T: Texture2D<Pixel = P>,
{
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
impl<P, T> CubeTexture<P, T>
where
  T: Texture2dSampleAble<Pixel = P>,
{
  pub fn sample<S: Scalar>(
    &self,
    _direction: NormalizedVector<S, Vec3<S>>,
    // filter: TextureFilterMode,
  ) -> P {
    todo!()
    // let abs = direction.map(|c| c.abs());
    // let max_axis_project = abs.x.max(abs.y).max(abs.z);
    // let dir = direction / max_axis_project;
    // let re_range = |v: S| (v + S::one()) * S::half();
    // if dir.x == S::one() {
    //   let at = Vec2::new(dir.y, dir.z).map(re_range);
    //   self.positive_x.sample(at)
    // //
    // } else if dir.x == -S::one() {
    //   //
    // } else if dir.y == S::one() {
    //   //
    // } else if dir.y == -S::one() {
    //   //
    // } else if dir.z == S::one() {
    //   //
    // } else {
    //   //
    // }
  }
}
