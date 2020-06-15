use crate::math::{PI, Vec3};

// http://www.codinglabs.net/article_physically_based_rendering_cook_torrance.aspx
// https://blog.selfshadow.com/publications/s2012-shading-course/burley/s2012_pbs_disney_brdf_notes_v3.pdf
// https://blog.uwa4d.com/archives/1582.html
pub trait CookTorranceBRDF {
  // micro normal distribution
  fn d(&self, n: Vec3, h: Vec3) -> f32;
  // geometric shadow
  fn g(&self, l: Vec3, v: Vec3, n: Vec3) -> f32;
  // fresnel
  fn f(&self, v: Vec3, h: Vec3) -> f32;

  fn evaluate(&self, l: Vec3, v: Vec3, n: Vec3) -> f32{
    let h = (l + v).normalize();
    (self.d(n, h) * self.g(l, v, n) * self.f(v, h)) / (4.0 * n.dot(l) * n.dot(v))
  }
}


pub fn f_schlick( v: Vec3, h: Vec3, f0: f32) -> f32 {
  f0 + (1.0 - f0) * (1.0 - v.dot(h)).powi(5)
}

pub fn saturate(v: f32) -> f32 {
  v.min(1.0).max(0.0)
}

pub struct BlinnPhong{
  shininess: f32
}

impl CookTorranceBRDF for BlinnPhong{
  fn d(&self, n: Vec3, h: Vec3) -> f32{
    let normalize_coefficient =  (self.shininess + 2.0) / 2.0 * PI;
    let cos = n.dot(h);
    saturate(cos).powf(self.shininess) * normalize_coefficient
  }
  fn g(&self, l: Vec3, v: Vec3, n: Vec3) -> f32{
    4.0 * n.dot(l) * n.dot(v)
  }
  fn f(&self, v: Vec3, h: Vec3) -> f32{
    1.0
  }
}

// impl CookTorranceBRDF for CookTorrance {

// }