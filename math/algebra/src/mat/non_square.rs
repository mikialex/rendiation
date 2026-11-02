//! The non square matrices, named as MatCxR (C columns and R rows) like WGSL. They are mainly
//! used as the shader data types, so only the basic component-wise operations are provided, except
//! the [`Mat4x3`] which is also used as the compact affine transform.

use std::ops::{Add, Mul};

use crate::*;

macro_rules! impl_non_square_matrix {
  ($MatrixN: ident, $rows: expr, { $($column: ident: [$($field: ident),+]),+ }) => {
    /// The column major non square matrix, see the module document.
    #[repr(C)]
    #[derive(Serialize, Deserialize)]
    #[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
    pub struct $MatrixN<T> {
      $($(pub $field: T,)+)+
    }

    unsafe impl<T: bytemuck::Zeroable> bytemuck::Zeroable for $MatrixN<T> {}
    unsafe impl<T: bytemuck::Pod> bytemuck::Pod for $MatrixN<T> {}

    impl<T> $MatrixN<T> {
      /// Create the matrix from the columns.
      pub fn new_from_column($($column: [T; $rows]),+) -> Self {
        $(let [$($field),+] = $column;)+
        Self { $($($field),+),+ }
      }
    }
  };
}

impl_non_square_matrix!(Mat2x3, 3, { a: [a1, a2, a3], b: [b1, b2, b3] });
impl_non_square_matrix!(Mat2x4, 4, { a: [a1, a2, a3, a4], b: [b1, b2, b3, b4] });
impl_non_square_matrix!(Mat3x2, 2, { a: [a1, a2], b: [b1, b2], c: [c1, c2] });
impl_non_square_matrix!(Mat3x4, 4, {
  a: [a1, a2, a3, a4],
  b: [b1, b2, b3, b4],
  c: [c1, c2, c3, c4]
});
impl_non_square_matrix!(Mat4x2, 2, { a: [a1, a2], b: [b1, b2], c: [c1, c2], d: [d1, d2] });
impl_non_square_matrix!(Mat4x3, 3, {
  a: [a1, a2, a3],
  b: [b1, b2, b3],
  c: [c1, c2, c3],
  d: [d1, d2, d3]
});

impl<T> Mat4x3<T> {
  /// The linear part of the affine transform.
  pub fn to_mat3(self) -> Mat3<T> {
    #[rustfmt::skip]
    Mat3 {
      a1: self.a1, a2: self.a2, a3: self.a3,
      b1: self.b1, b2: self.b2, b3: self.b3,
      c1: self.c1, c2: self.c2, c3: self.c3,
    }
  }
}

impl<T: Scalar> Mat4x3<T> {
  /// The matrix to transform the normal vector, see [`Mat4::to_normal_matrix`].
  pub fn to_normal_matrix(self) -> Mat3<T> {
    self.to_mat3().inverse_or_identity().transpose()
  }
}

/// Transform the 3d point by the affine transform.
impl<T> Mul<Vec3<T>> for Mat4x3<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T>,
{
  type Output = Vec3<T>;

  fn mul(self, v: Vec3<T>) -> Vec3<T> {
    Vec3 {
      x: v.x * self.a1 + v.y * self.b1 + v.z * self.c1 + self.d1,
      y: v.x * self.a2 + v.y * self.b2 + v.z * self.c2 + self.d2,
      z: v.x * self.a3 + v.y * self.b3 + v.z * self.c3 + self.d3,
    }
  }
}

/// The plain matrix vector multiplication, same as WGSL: `mat4x3 * vec4 = vec3`.
impl<T> Mul<Vec4<T>> for Mat4x3<T>
where
  T: Copy + Add<Output = T> + Mul<Output = T>,
{
  type Output = Vec3<T>;

  fn mul(self, v: Vec4<T>) -> Vec3<T> {
    Vec3 {
      x: v.x * self.a1 + v.y * self.b1 + v.z * self.c1 + v.w * self.d1,
      y: v.x * self.a2 + v.y * self.b2 + v.z * self.c2 + v.w * self.d2,
      z: v.x * self.a3 + v.y * self.b3 + v.z * self.c3 + v.w * self.d3,
    }
  }
}

#[rustfmt::skip]
impl<T> Mat4x3<T> {
  /// Create the matrix column by column: `m11, m12, m13` is the first column.
  pub const fn new(
    m11: T, m12: T, m13: T,
    m21: T, m22: T, m23: T,
    m31: T, m32: T, m33: T,
    m41: T, m42: T, m43: T,
  ) -> Self {
    Self {
      a1: m11, a2: m12, a3: m13,
      b1: m21, b2: m22, b3: m23,
      c1: m31, c2: m32, c3: m33,
      d1: m41, d2: m42, d3: m43,
    }
  }
}

impl<T> Mat4x3<T>
where
  T: Scalar,
{
  /// See [`Mat4::from_orth_basis_and_position`].
  pub fn from_orth_basis_and_position(forward: Vec3<T>, up: Vec3<T>, position: Vec3<T>) -> Self {
    let right = forward.cross(up);
    #[rustfmt::skip]
    Mat4x3::new(
      right.x,    right.y,    right.z,
      up.x,       up.y,       up.z,
      forward.x,  forward.y,  forward.z,
      position.x, position.y,  position.z,
    )
  }

  /// The first column, the transformed x axis.
  pub fn right(&self) -> Vec3<T> {
    Vec3::new(self.a1, self.a2, self.a3)
  }

  /// The second column, the transformed y axis.
  pub fn up(&self) -> Vec3<T> {
    Vec3::new(self.b1, self.b2, self.b3)
  }

  /// The third column, the transformed z axis.
  pub fn forward(&self) -> Vec3<T> {
    Vec3::new(self.c1, self.c2, self.c3)
  }

  /// The translation.
  pub fn position(&self) -> Vec3<T> {
    Vec3::new(self.d1, self.d2, self.d3)
  }

  /// The length of each transformed axis.
  pub fn get_scale(&self) -> Vec3<T> {
    let sx = Vec3::new(self.a1, self.a2, self.a3).length();
    let sy = Vec3::new(self.b1, self.b2, self.b3).length();
    let sz = Vec3::new(self.c1, self.c2, self.c3).length();
    Vec3::new(sx, sy, sz)
  }
}

/// Drop the last row, the matrix must be affine (the last row is `0, 0, 0, 1`).
impl<T: Scalar> From<Mat4<T>> for Mat4x3<T> {
  fn from(m: Mat4<T>) -> Self {
    debug_assert!(
      m.a4 == T::zero() && m.b4 == T::zero() && m.c4 == T::zero() && m.d4 == T::one(),
      "the matrix is not affine: {m:?}"
    );
    #[rustfmt::skip]
    Self {
      a1: m.a1, a2: m.a2, a3: m.a3,
      b1: m.b1, b2: m.b2, b3: m.b3,
      c1: m.c1, c2: m.c2, c3: m.c3,
      d1: m.d1, d2: m.d2, d3: m.d3,
    }
  }
}

/// Use the 3d linear transform as the linear part, the translation is zero.
impl<T: Scalar> From<Mat3<T>> for Mat4x3<T> {
  fn from(m: Mat3<T>) -> Self {
    #[rustfmt::skip]
    Self {
      a1: m.a1,      a2: m.a2,      a3: m.a3,
      b1: m.b1,      b2: m.b2,      b3: m.b3,
      c1: m.c1,      c2: m.c2,      c3: m.c3,
      d1: T::zero(), d2: T::zero(), d3: T::zero(),
    }
  }
}
