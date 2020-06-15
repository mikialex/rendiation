use crate::math::Vec3;

// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
// https://blog.uwa4d.com/archives/1582.html
pub trait CookTorranceBRDF {
  // micro normal distribution
  fn d(&self, h: Vec3) -> f32;
  // geometric shadow
  fn g(&self, l: Vec3, v: Vec3, h: Vec3) -> f32;
  // fresnel
  fn f(&self, v: Vec3, h: Vec3) -> f32;

  fn evaluate(&self, l: Vec3, v: Vec3, n: Vec3) -> f32{
    let h = (l + v).normalize();
    (self.d(h) * self.g(l, v, h) * self.f(v, h)) / (4.0 * n.dot(l) * n.dot(v))
  }
}


pub fn f_schlick( v: Vec3, h: Vec3, f0: f32) -> f32 {
  f0 + (1.0 - f0) * (1.0 - v.dot(h)).powi(5)
}