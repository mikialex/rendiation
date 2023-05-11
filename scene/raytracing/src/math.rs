use rendiation_algebra::IntoNormalizedVector;
use rendiation_algebra::{InnerProductSpace, NormalizedVector, Vec2, Vec3};

pub type NormalizedVec3<T> = NormalizedVector<T, Vec3<T>>;
pub use rand as randx;
pub use rendiation_geometry::*;

use crate::Sampler;

pub fn rand() -> f32 {
  randx::random()
}

pub const PI_OVER_4: f32 = std::f32::consts::PI / 4.0;
pub const PI_OVER_2: f32 = std::f32::consts::PI / 2.0;
pub const PI: f32 = std::f32::consts::PI;
pub const INV_PI: f32 = 1.0 / std::f32::consts::PI;

/// http://l2program.co.uk/900/concentric-disk-sampling
/// Uniformly distribute samples over a unit disk.
pub fn concentric_sample_disk(sampler: &mut (impl Sampler + ?Sized)) -> Vec2<f32> {
  // map uniform random numbers to $[-1,1]^2$s
  let u_offset = sampler.next_2d_vec() * 2.0 - Vec2::new(1.0, 1.0);
  // handle degeneracy at the origin
  if u_offset.x == 0.0 && u_offset.y == 0.0 {
    return Vec2::new(0.0, 0.0);
  }
  // apply concentric mapping to point
  let theta: f32;
  let r: f32;
  if u_offset.x.abs() > u_offset.y.abs() {
    r = u_offset.x;
    theta = PI_OVER_4 * (u_offset.y / u_offset.x);
  } else {
    r = u_offset.y;
    theta = PI_OVER_2 - PI_OVER_4 * (u_offset.x / u_offset.y);
  }
  Vec2::new(theta.cos(), theta.sin()) * r
}

pub fn cosine_sample_hemisphere(sampler: &mut (impl Sampler + ?Sized)) -> Vec3<f32> {
  let d = concentric_sample_disk(sampler);
  let z = 0.0_f32.max(1.0 - d.x * d.x - d.y * d.y).sqrt();
  Vec3::new(d.x, d.y, z)
}

pub fn cosine_sample_hemisphere_in_dir(
  dir: NormalizedVec3<f32>,
  sampler: &mut (impl Sampler + ?Sized),
) -> (NormalizedVec3<f32>, f32) {
  let offset = cosine_sample_hemisphere(sampler);

  let left = Vec3::new(0.0, 1.0, 0.0).cross(*dir).normalize();
  let up = left.cross(*dir);

  let xy_r = (offset.x * offset.x + offset.y * offset.y).sqrt();
  if xy_r == 0. {
    return (dir, 1.);
  }
  let cos_phi = offset.x / xy_r;
  let sin_phi = offset.y / xy_r;
  let cos_theta = offset.z;
  let sin_theta = xy_r;

  (
    (left * sin_theta * cos_phi + up * sin_theta * sin_phi + dir * cos_theta).into_normalized(),
    cos_theta,
  )
}

/// Uniformly sample a direction on the unit sphere about the origin
pub fn uniform_sample_sphere_dir(sampler: &mut (impl Sampler + ?Sized)) -> NormalizedVec3<f32> {
  let (sample_x, sample_y) = sampler.next_2d();
  let z = 1.0 - 2.0 * sample_x;
  let r = f32::sqrt(f32::max(0.0, 1.0 - z * z));
  let phi = PI * 2.0 * sample_y;
  let dir = Vec3::new(f32::cos(phi) * r, f32::sin(phi) * r, z);
  unsafe { dir.into_normalized_unchecked() }
}
