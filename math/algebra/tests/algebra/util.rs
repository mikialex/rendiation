use glam::{DMat2, DMat3, DMat4, DQuat, DVec2, DVec3, DVec4};
use rendiation_algebra::*;

pub const EPS: f64 = 1e-9;

/// A deterministic xorshift random generator, so the tests are reproducible without extra deps.
pub struct Rng(u64);

impl Rng {
  pub fn new(seed: u64) -> Self {
    Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
  }

  /// uniform in -1 to 1
  pub fn f(&mut self) -> f64 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    (self.0 >> 11) as f64 / (1u64 << 52) as f64 - 1.0
  }

  pub fn range(&mut self, min: f64, max: f64) -> f64 {
    min + (self.f() * 0.5 + 0.5) * (max - min)
  }

  pub fn vec2(&mut self) -> Vec2<f64> {
    Vec2::new(self.f(), self.f()) * 10.
  }

  pub fn vec3(&mut self) -> Vec3<f64> {
    Vec3::new(self.f(), self.f(), self.f()) * 10.
  }

  pub fn vec4(&mut self) -> Vec4<f64> {
    Vec4::new(self.f(), self.f(), self.f(), self.f()) * 10.
  }

  pub fn unit_vec3(&mut self) -> Vec3<f64> {
    loop {
      let v = Vec3::new(self.f(), self.f(), self.f());
      if v.length() > 0.1 {
        return v.normalize();
      }
    }
  }

  pub fn angle(&mut self) -> f64 {
    self.f() * std::f64::consts::PI
  }

  pub fn quat(&mut self) -> Quat<f64> {
    Quat::rotation(self.unit_vec3(), self.angle())
  }

  pub fn mat2(&mut self) -> Mat2<f64> {
    std::array::from_fn(|_| self.f()).into()
  }

  pub fn mat3(&mut self) -> Mat3<f64> {
    std::array::from_fn(|_| self.f()).into()
  }

  pub fn mat4(&mut self) -> Mat4<f64> {
    std::array::from_fn(|_| self.f()).into()
  }

  pub fn scale3(&mut self) -> Vec3<f64> {
    Vec3::new(
      self.range(0.2, 3.),
      self.range(0.2, 3.),
      self.range(0.2, 3.),
    )
  }

  /// random translation * rotation * scale
  pub fn trs(&mut self) -> Mat4<f64> {
    Mat4::compose(self.vec3(), self.quat(), self.scale3())
  }
}

pub fn for_each_seed(f: impl Fn(&mut Rng)) {
  for seed in 0..64 {
    f(&mut Rng::new(seed + 1));
  }
}

pub fn assert_near_slice(a: &[f64], b: &[f64], eps: f64) {
  assert_eq!(a.len(), b.len());
  for (x, y) in a.iter().zip(b) {
    let tolerance = eps * (1.0 + x.abs().max(y.abs()));
    assert!((x - y).abs() <= tolerance, "{a:?} != {b:?}");
  }
}

/// The value types that can be compared with glam's counterparts.
pub trait ToGlam {
  type Glam;
  fn to_glam(self) -> Self::Glam;
}

/// Compare as the flattened f64 elements.
pub trait Elements {
  fn elements(&self) -> Vec<f64>;
}

macro_rules! glam_mapping {
  ($ty: ident, $glam: ty, $n: expr, $from: expr, $to: ident) => {
    impl ToGlam for $ty<f64> {
      type Glam = $glam;
      fn to_glam(self) -> $glam {
        let arr: [f64; $n] = self.into();
        $from(&arr)
      }
    }
    impl Elements for $ty<f64> {
      fn elements(&self) -> Vec<f64> {
        let arr: [f64; $n] = (*self).into();
        arr.to_vec()
      }
    }
    impl Elements for $glam {
      fn elements(&self) -> Vec<f64> {
        self.$to().to_vec()
      }
    }
  };
}

glam_mapping!(Vec2, DVec2, 2, DVec2::from_slice, to_array);
glam_mapping!(Vec3, DVec3, 3, DVec3::from_slice, to_array);
glam_mapping!(Vec4, DVec4, 4, DVec4::from_slice, to_array);
glam_mapping!(Mat2, DMat2, 4, DMat2::from_cols_array, to_cols_array);
glam_mapping!(Mat3, DMat3, 9, DMat3::from_cols_array, to_cols_array);
glam_mapping!(Mat4, DMat4, 16, DMat4::from_cols_array, to_cols_array);
glam_mapping!(Quat, DQuat, 4, DQuat::from_slice, to_array);

impl Elements for f64 {
  fn elements(&self) -> Vec<f64> {
    vec![*self]
  }
}

#[track_caller]
pub fn assert_near(a: impl Elements, b: impl Elements) {
  assert_near_slice(&a.elements(), &b.elements(), EPS);
}

#[track_caller]
pub fn assert_near_eps(a: impl Elements, b: impl Elements, eps: f64) {
  assert_near_slice(&a.elements(), &b.elements(), eps);
}

/// The quaternion q and -q represent the same rotation.
#[track_caller]
pub fn assert_same_rotation(a: Quat<f64>, b: DQuat) {
  let b = if a.dot(Quat::new(b.x, b.y, b.z, b.w)) < 0. {
    -b
  } else {
    b
  };
  assert_near_eps(a, b, 1e-7);
}
