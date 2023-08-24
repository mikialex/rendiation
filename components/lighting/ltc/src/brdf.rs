use crate::*;

pub struct BrdfEval {
  pub value: f32,
  pub pdf: f32,
}

pub trait Brdf: Copy {
  fn eval(&self, v: Vec3<f32>, l: Vec3<f32>, alpha: f32) -> BrdfEval;
  fn sample(&self, v: Vec3<f32>, alpha: f32, u1: f32, u2: f32) -> Vec3<f32>;
}

#[derive(Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
struct GGX;

impl Brdf for GGX {
  fn eval(&self, v: Vec3<f32>, l: Vec3<f32>, alpha: f32) -> BrdfEval {
    if v.z <= 0. {
      return BrdfEval { value: 0., pdf: 0. };
    }

    // masking
    let lambda_v = lambda(alpha, v.z);

    // shadowing
    let g2 = if l.z <= 0.0 {
      0.
    } else {
      let lambda_l = lambda(alpha, l.z);
      1.0 / (1.0 + lambda_v + lambda_l)
    };

    // D
    let h = (v + l).normalize();
    let slopex = h.x / h.z;
    let slopey = h.y / h.z;
    let mut d = 1.0 / (1.0 + (slopex * slopex + slopey * slopey) / alpha / alpha);
    d *= d;
    d /= f32::PI() * alpha * alpha * h.z * h.z * h.z * h.z;

    let pdf = (d * h.z / 4.0 / v.dot(h)).abs();
    let value = d * g2 / 4.0 / v.z;

    BrdfEval { value, pdf }
  }

  fn sample(&self, v: Vec3<f32>, alpha: f32, u1: f32, u2: f32) -> Vec3<f32> {
    let phi = 2.0 * f32::PI() * u1;
    let r = alpha * (u2 / (1.0 - u2)).sqrt();
    let n = Vec3::new(r * phi.cos(), r * phi.sin(), 1.0).normalize();
    v.reverse() + 2.0 * n * n.dot(v)
  }
}

fn lambda(alpha: f32, cos_theta: f32) -> f32 {
  let a = 1. / alpha / cos_theta.acos().tan();
  if cos_theta < 1. {
    0.5 * (-1.0 + (1.0 + 1.0 / a / a).sqrt())
  } else {
    0.
  }
}
