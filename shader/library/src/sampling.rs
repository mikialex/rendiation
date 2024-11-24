use crate::*;

/// http://holger.dammertz.org/stuff/notes_HammersleyOnHemisphere.html

/// generate an unbound 1D white noise by a 2D seed(such as uv)
#[shader_fn]
pub fn random(seed: Node<Vec2<f32>>) -> Node<f32> {
  let s1 = val(12.9898);
  let s2 = val(78.233);
  let s3 = val(43758.545);
  (seed.dot((s1, s2)).sin() * s3).fract()
}

/// generate an unbound 3D white noise by a 2D seed(such as uv)
#[shader_fn]
pub fn random3(seed: Node<Vec2<f32>>) -> Node<Vec3<f32>> {
  let x = random(seed);
  let y = random((seed + random(seed).splat()).sin());
  let z = random(seed + random(seed).cos().splat() + random(seed).splat());
  (x, y, z).into()
}

/// generate a low discrepancy uniform distribution in 2D, bounded in [0., 1.]
#[shader_fn]
pub fn hammersley_2d(index: Node<u32>, total_num_samples: Node<u32>) -> Node<Vec2<f32>> {
  (
    index.into_f32() / total_num_samples.into_f32(),
    radical_inverse_vdc(index),
  )
    .into()
}

#[shader_fn]
fn radical_inverse_vdc(bits: Node<u32>) -> Node<f32> {
  let bits = (bits << val(16)) | (bits >> val(16));
  let bits = ((bits & val(0x55555555)) << val(1)) | ((bits & val(0xAAAAAAAA)) >> val(1));
  let bits = ((bits & val(0x33333333)) << val(2)) | ((bits & val(0xCCCCCCCC)) >> val(2));
  let bits = ((bits & val(0x0F0F0F0F)) << val(4)) | ((bits & val(0xF0F0F0F0)) >> val(4));
  let bits = ((bits & val(0x00FF00FF)) << val(8)) | ((bits & val(0xFF00FF00)) >> val(8));

  bits.into_f32() * val(2.328_306_4e-10) // 0x100000000
}

/// map the distribution from the unit square to unit sphere uniformly
#[shader_fn]
pub fn sample_hemisphere_uniform(uv: Node<Vec2<f32>>) -> Node<Vec3<f32>> {
  let phi = val(2.0 * std::f32::consts::PI) * uv.y();
  let cos_theta = val(1.0) - uv.x();
  let sin_theta = (val(1.0) - cos_theta * cos_theta).sqrt();
  (phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta).into()
}

/// map the distribution from the unit square to unit sphere by cos weight
#[shader_fn]
pub fn sample_hemisphere_cos(uv: Node<Vec2<f32>>) -> Node<Vec3<f32>> {
  let phi = val(2.0 * std::f32::consts::PI) * uv.y();
  let cos_theta = (val(1.0) - uv.x()).sqrt();
  let sin_theta = (val(1.0) - cos_theta * cos_theta).sqrt();
  (phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta).into()
}

/// https://graphics.pixar.com/library/OrthonormalB/paper.pdf
#[shader_fn]
pub fn tbn(normal: Node<Vec3<f32>>) -> Node<Mat3<f32>> {
  let sign = normal.z().less_than(0.).select(val(-1.), val(1.));
  let a = val(-1.) / (sign + normal.z());
  let b = normal.x() * normal.y() * a;
  let tangent = vec3_node((
    val(1.) + sign * normal.x() * normal.y() * a,
    sign * b,
    -sign * normal.x(),
  ));
  let bi_tangent = vec3_node((b, sign + normal.y() * normal.y() * a, -normal.y()));
  (tangent.normalize(), bi_tangent.normalize(), normal).into()
}
