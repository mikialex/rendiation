use rendiation_algebra::{Lerp, NormalizedVector, Scalar, Vec2, Vec3};

use crate::{AddressMode, FilterMode, Texture2D, Texture2dSampleAble};

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

// http://www.cim.mcgill.ca/~langer/557/18-slides.pdf
impl<P, T> CubeTexture<P, T>
where
  T: Texture2dSampleAble<Pixel = P>,
{
  pub fn sample<S>(
    &self,
    direction: NormalizedVector<S, Vec3<S>>,
    filter: FilterMode,
    address: AddressMode,
  ) -> P
  where
    S: Scalar + From<usize> + Into<usize>,
    P: Lerp<S>,
  {
    let abs = direction.map(|c| c.abs());
    let max_axis_project = abs.x.max(abs.y).max(abs.z);
    let dir = direction / max_axis_project;

    let (face, at) = if dir.x == S::one() {
      (&self.positive_x, (dir.y, dir.z))
    } else if dir.x == -S::one() {
      (&self.negative_x, (dir.y, dir.z))
    }
    //
    else if dir.y == S::one() {
      (&self.positive_y, (dir.x, dir.z))
    } else if dir.y == -S::one() {
      (&self.negative_y, (dir.x, dir.z))
    }
    //
    else if dir.z == S::one() {
      (&self.positive_z, (dir.x, dir.y))
    } else {
      (&self.negative_z, (dir.x, dir.y))
    };

    let re_range = |v: S| (v + S::one()) * S::half();
    let pixel_position = Vec2::from(at).map(re_range);
    face.sample_dyn(pixel_position, address, filter)
  }
}
