#[cfg(test)]
mod tests {
  use crate::*;
  use approx::{abs_diff_eq, AbsDiffEq};

  impl<T: AbsDiffEq, const N: usize> AbsDiffEq for Vector<T, { N }>
  where
    T::Epsilon: Copy,
  {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> T::Epsilon {
      T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: T::Epsilon) -> bool {
      self
        .iter()
        .zip(other.iter())
        .all(|(x, y)| T::abs_diff_eq(x, y, epsilon))
    }
  }

  impl<T: AbsDiffEq, const N: usize, const M: usize> AbsDiffEq for Matrix<T, { N }, { M }>
  where
    T::Epsilon: Copy,
  {
    type Epsilon = T::Epsilon;

    fn default_epsilon() -> T::Epsilon {
      T::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: T::Epsilon) -> bool {
      self
        .column_iter()
        .zip(other.column_iter())
        .all(|(x, y)| Vector::<T, { N }>::abs_diff_eq(x, y, epsilon))
    }
  }

  type Vector1<T> = Vector<T, 1>;

  /*
  #[test]
  fn test_permutation() {
      let p1 = Permutation::unit();
      let p2 = Permutation([0usize, 1, 2]);
      let p3 = Permutation([1usize, 2, 0]);
      let v = vector!(1.0f64, 2.0, 3.0);
      assert_eq!(p1, p2);
      assert_eq!(v, p3 * (p3 * (p3 * v)));
  }

  #[test]
  fn test_permutation_parity() {
      let p1 = Permutation::<4>::unit();
      let p2 = Permutation([3usize, 1, 2, 0]);
      let p3 = Permutation([2usize, 3, 1, 0]);
      assert!(!p1.odd_parity());
      assert!(p2.odd_parity());
      assert!(p3.odd_parity());
  }
  */

  #[test]
  fn test_vec_zero() {
    let a = Vector3::<u32>::zero();
    assert_eq!(a, Vector3::<u32>::from([0, 0, 0]));
  }

  #[test]
  fn test_vec_index() {
    let a = Vector1::<u32>::from([0]);
    assert_eq!(a[0], 0);
    let mut b = Vector2::<u32>::from([1, 2]);
    b[1] += 3;
    assert_eq!(b[1], 5);
  }

  #[test]
  fn test_vec_eq() {
    let a = Vector1::<u32>::from([0]);
    let b = Vector1::<u32>::from([1]);
    let c = Vector1::<u32>::from([0]);
    let d = [0u32];
    assert_ne!(a, b);
    assert_eq!(a, c);
    assert_eq!(a, &d); // No blanket impl on T for deref... why? infinite
                       // loops?
  }

  #[test]
  fn test_vec_addition() {
    let a = Vector1::<u32>::from([0]);
    let b = Vector1::<u32>::from([1]);
    let c = Vector1::<u32>::from([2]);
    assert_eq!(a + b, b);
    assert_eq!(b + b, c);
    // We shouldn't need to have to test more dimensions, but we shall test
    // one more.
    let a = Vector2::<u32>::from([0, 1]);
    let b = Vector2::<u32>::from([1, 2]);
    let c = Vector2::<u32>::from([1, 3]);
    let d = Vector2::<u32>::from([2, 5]);
    assert_eq!(a + b, c);
    assert_eq!(b + c, d);
    let mut c = Vector2::<u32>::from([1, 3]);
    let d = Vector2::<u32>::from([2, 5]);
    c += d;
    let e = Vector2::<u32>::from([3, 8]);
    assert_eq!(c, e);
  }

  #[test]
  fn test_vec_subtraction() {
    let mut a = Vector1::<u32>::from([3]);
    let b = Vector1::<u32>::from([1]);
    let c = Vector1::<u32>::from([2]);
    assert_eq!(a - c, b);
    a -= b;
    assert_eq!(a, c);
  }

  #[test]
  fn test_vec_negation() {
    let a = Vector4::<i32>::from([1, 2, 3, 4]);
    let b = Vector4::<i32>::from([-1, -2, -3, -4]);
    assert_eq!(-a, b);
  }

  #[test]
  fn test_vec_scale() {
    let a = Vector4::<f32>::from([2.0, 4.0, 2.0, 4.0]);
    let b = Vector4::<f32>::from([4.0, 8.0, 4.0, 8.0]);
    let c = Vector4::<f32>::from([1.0, 2.0, 1.0, 2.0]);
    assert_eq!(a * 2.0, b);
    assert_eq!(a / 2.0, c);
  }

  #[test]
  fn test_vec_cross() {
    let a = vector!(1isize, 2isize, 3isize);
    let b = vector!(4isize, 5isize, 6isize);
    let r = vector!(-3isize, 6isize, -3isize);
    assert_eq!(a.cross(b), r);
  }

  #[test]
  fn test_vec_distance() {
    let a = Vector1::<f32>::from([0.0]);
    let b = Vector1::<f32>::from([1.0]);
    assert_eq!(a.distance2(b), 1.0);
    let a = Vector1::<f32>::from([0.0]);
    let b = Vector1::<f32>::from([2.0]);
    assert_eq!(a.distance2(b), 4.0);
    assert_eq!(a.distance(b), 2.0);
    let a = Vector2::<f32>::from([0.0, 0.0]);
    let b = Vector2::<f32>::from([1.0, 1.0]);
    assert_eq!(a.distance2(b), 2.0);
  }

  #[test]
  fn test_vec_normalize() {
    let a = vector!(5.0);
    assert_eq!(a.clone().magnitude(), 5.0);
    let a_norm = a.normalize();
    assert_eq!(a_norm, vector!(1.0));
  }

  #[test]
  fn test_vec_transpose() {
    let v = vector!(1i32, 2, 3, 4);
    let m = Matrix::<i32, 1, 4>::from([vector!(1i32), vector!(2), vector!(3), vector!(4)]);
    assert_eq!(v.transpose(), m);
  }

  #[test]
  fn test_from_fn() {
    let indices: Vector<usize, 10> = vector!(0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    assert_eq!(Vector::<usize, 10>::from_fn(|i| i), indices);
  }

  #[test]
  fn test_decompose() {
    let a = matrix![[-1.0f64, 1.0], [2.0, 1.0]];
    let b = vector!(5.0f64, 2.0);
    let lu = a.lu().unwrap();

    assert_eq!(a * lu.solve(b), b);
  }

  #[test]
  fn test_vec_map() {
    let int = vector!(1i32, 0, 1, 1, 0, 1, 1, 0, 0, 0);
    let boolean = vector!(true, false, true, true, false, true, true, false, false, false);
    assert_eq!(int.map(|i| i != 0), boolean);
  }

  #[test]
  fn test_vec_from_iter() {
    let v = vec![1i32, 2, 3, 4];
    let vec = Vector::<i32, 4>::from_iter(v);
    assert_eq!(vec, vector![1i32, 2, 3, 4])
  }

  #[test]
  fn test_vec_into_iter() {
    let v = vector!(1i32, 2, 3, 4);
    let vec: Vec<i32> = v.into_iter().collect();
    assert_eq!(vec, vec![1i32, 2, 3, 4])
  }

  #[test]
  fn test_vec_indexed_map() {
    let boolean = vector!(true, false, true, true, false, true, true, false, false, false);
    let indices = vector!(0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    assert_eq!(boolean.indexed_map(|i, _| i), indices);
  }

  // Does not compile.
  /*
  #[test]
  fn test_vec_first() {
      let a = Vector2::<i32>::from([ 1, 2 ]);
      let b = Vector3::<i32>::from([ 1, 2, 3 ]);
      let c = b.first::<2_usize>();
      assert_eq!(a, c);
  }
  */

  #[test]
  fn test_mat_identity() {
    let unit = matrix![[1u32, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1],];
    assert_eq!(Matrix::<u32, 4, 4>::one(), unit);
  }

  #[test]
  fn test_mat_negation() {
    let neg_unit = matrix![
      [-1i32, 0, 0, 0],
      [0, -1, 0, 0],
      [0, 0, -1, 0],
      [0, 0, 0, -1],
    ];
    assert_eq!(-Matrix::<i32, 4, 4>::one(), neg_unit);
  }

  #[test]
  fn test_mat_add() {
    let a = matrix![matrix![1u32]];
    let b = matrix![matrix![10u32]];
    let c = matrix![matrix![11u32]];
    assert_eq!(a + b, c);
  }

  #[test]
  fn test_mat_scalar_mult() {
    let a = Matrix::<f32, 2, 2>::from([vector!(0.0, 1.0), vector!(0.0, 2.0)]);
    let b = Matrix::<f32, 2, 2>::from([vector!(0.0, 2.0), vector!(0.0, 4.0)]);
    assert_eq!(a * 2.0, b);
  }

  #[test]
  fn test_mat_mult() {
    let a = Matrix::<f32, 2, 2>::from([vector!(0.0, 0.0), vector!(1.0, 0.0)]);
    let b = Matrix::<f32, 2, 2>::from([vector!(0.0, 1.0), vector!(0.0, 0.0)]);
    assert_eq!(a * b, matrix![[1.0, 0.0], [0.0, 0.0],]);
    assert_eq!(b * a, matrix![[0.0, 0.0], [0.0, 1.0],]);
    // Basic example:
    let a: Matrix<usize, 1, 1> = matrix![1];
    let b: Matrix<usize, 1, 1> = matrix![2];
    let c: Matrix<usize, 1, 1> = matrix![2];
    assert_eq!(a * b, c);
    // Removing the type signature here caused the compiler to crash.
    // Since then I've been wary.
    let a = Matrix::<f32, 3, 3>::from([
      vector!(1.0, 0.0, 0.0),
      vector!(0.0, 1.0, 0.0),
      vector!(0.0, 0.0, 1.0),
    ]);
    let b = a.clone();
    let c = a * b;
    assert_eq!(
      c,
      matrix![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0],]
    );
    // Here is another random example I found online.
    let a: Matrix<i32, 3, 3> = matrix![[0, -3, 5], [6, 1, -4], [2, 3, -2],];
    let b: Matrix<i32, 3, 3> = matrix![[-1, 0, -3], [4, 5, 1], [2, 6, -2]];
    let c: Matrix<i32, 3, 3> = matrix![[-2, 15, -13], [-10, -19, -9], [6, 3, 1]];
    assert_eq!(a * b, c);
  }

  #[test]
  fn test_mat_index() {
    let m: Matrix<i32, 2, 2> = matrix![[0, 2], [1, 3],];
    assert_eq!(m[(0, 0)], 0);
    assert_eq!(m[0][0], 0);
    assert_eq!(m[(1, 0)], 1);
    assert_eq!(m[0][1], 1);
    assert_eq!(m[(0, 1)], 2);
    assert_eq!(m[1][0], 2);
    assert_eq!(m[(1, 1)], 3);
    assert_eq!(m[1][1], 3);
  }

  #[test]
  fn test_mat_transpose() {
    assert_eq!(
      Matrix::<i32, 1, 2>::from([vector!(1), vector!(2)]).transpose(),
      Matrix::<i32, 2, 1>::from([vector!(1, 2)])
    );
    assert_eq!(
      matrix![[1, 2], [3, 4],].transpose(),
      matrix![[1, 3], [2, 4],]
    );
  }

  #[test]
  fn test_square_matrix() {
    let a: Matrix<i32, 3, 3> = matrix![[5, 0, 0], [0, 8, 12], [0, 0, 16],];
    let diag: Vector<i32, 3> = vector!(5, 8, 16);
    assert_eq!(a.diagonal(), diag);
  }

  #[test]
  fn test_readme_code() {
    let a = vector!(0u32, 1, 2, 3);
    assert_eq!(a, Vector::<u32, 4>::from([0u32, 1, 2, 3]));

    let b = Vector::<f32, 7>::from([0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let c = Vector::<f32, 7>::from([1.0f32, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]) * 0.5;
    assert_eq!(
      b + c,
      Vector::<f32, 7>::from([0.5f32, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5])
    );

    let a = vector!(1i32, 1);
    let b = vector!(5i32, 5);
    assert_eq!(a.distance2(b), 32); // distance method not implemented.
    assert_eq!((b - a).magnitude2(), 32); // magnitude method not implemented.

    let a = vector!(1.0f32, 1.0);
    let b = vector!(5.0f32, 5.0);
    const CLOSE: f32 = 5.65685424949;
    assert_eq!(a.distance(b), CLOSE); // distance is implemented.
    assert_eq!((b - a).magnitude(), CLOSE); // magnitude is implemented.

    // Vector normalization is also supported for floating point scalars.
    assert_eq!(
      vector!(0.0f32, 20.0, 0.0).normalize(),
      vector!(0.0f32, 1.0, 0.0)
    );

    let _a = Matrix::<f32, 3, 3>::from([
      vector!(1.0, 0.0, 0.0),
      vector!(0.0, 1.0, 0.0),
      vector!(0.0, 0.0, 1.0),
    ]);
    let _b: Matrix<i32, 3, 3> = matrix![[0, -3, 5], [6, 1, -4], [2, 3, -2]];

    assert_eq!(
      matrix![[1i32, 0, 0,], [0, 2, 0], [0, 0, 3],].diagonal(),
      vector!(1i32, 2, 3)
    );

    assert_eq!(
      matrix![[1i32, 0, 0, 0], [0, 2, 0, 0], [0, 0, 3, 0], [0, 0, 0, 4]].diagonal(),
      vector!(1i32, 2, 3, 4)
    );
  }

  #[test]
  fn test_mat_map() {
    let int = matrix![[1i32, 0], [1, 1], [0, 1], [1, 0], [0, 0]];
    let boolean = matrix![
      [true, false],
      [true, true],
      [false, true],
      [true, false],
      [false, false]
    ];
    assert_eq!(int.map(|i| i != 0), boolean);
  }

  #[test]
  fn test_mat_from_iter() {
    let v = vec![1i32, 2, 3, 4];
    let mat = Matrix::<i32, 2, 2>::from_iter(v);
    assert_eq!(mat, matrix![[1i32, 2], [3, 4]].transpose())
  }

  #[test]
  fn test_mat_invert() {
    assert!(Matrix2::<f64>::one().invert().unwrap() == Matrix2::<f64>::one());

    // Example taken from cgmath:

    let a: Matrix2<f64> = matrix![[1.0f64, 2.0f64], [3.0f64, 4.0f64],];
    let identity: Matrix2<f64> = Matrix2::<f64>::one();
    abs_diff_eq!(
      a.invert().unwrap(),
      matrix![[-2.0f64, 1.0f64], [1.5f64, -0.5f64]]
    );

    abs_diff_eq!(a.invert().unwrap() * a, identity);
    abs_diff_eq!(a * a.invert().unwrap(), identity);
    assert!(matrix![[0.0f64, 2.0f64], [0.0f64, 5.0f64]]
      .invert()
      .is_none());
  }

  #[test]
  fn test_mat_determinant() {
    assert_eq!(Matrix2::<f64>::one().determinant(), f64::one());
    /*
    assert_eq!(
        matrix![[3.0f64, 8.0f64], [4.0f64, 6.0f64]].invert().unwrap(),
        matrix![[3.0f64, 8.0f64], [4.0f64, 6.0f64]]
    );
    */
    assert_eq!(
      matrix![[3.0f64, 8.0f64], [4.0f64, 6.0f64]].determinant(),
      -14.0f64
    );
    assert_eq!(
      matrix![[-2.0f64, 1.0f64], [1.5f64, -0.5f64]].determinant(),
      -0.5f64
    );
    assert_eq!(
      matrix![[6.0f64, 1.0, 1.0], [4.0, -2.0, 5.0], [2.0, 8.0, 7.0]].determinant(),
      -306.0f64
    );
  }

  #[test]
  fn test_mat_swap() {
    let mut m = matrix![[1.0, 2.0], [3.0, 4.0]];
    m.swap_columns(0, 1);
    assert_eq!(m, matrix![[2.0, 1.0], [4.0, 3.0]]);
    let mut m = matrix![[1.0, 2.0], [3.0, 4.0]];
    m.swap_rows(0, 1);
    assert_eq!(m, matrix![[3.0, 4.0], [1.0, 2.0]]);
    let mut m = matrix![[1.0, 2.0], [3.0, 4.0]];
    m.swap_columns(0, 0);
    assert_eq!(m, matrix![[1.0, 2.0], [3.0, 4.0]]);
    m.swap_rows(0, 0);
    assert_eq!(m, matrix![[1.0, 2.0], [3.0, 4.0]]);
  }

  #[test]
  fn test_vec_macro_constructor() {
    let v: Vector<f32, 0> = vector![];
    assert!(v.is_empty());

    let v = vector![1];
    assert_eq!(1, v[0]);

    let v = vector![1, 2, 3, 4, 5, 6, 7, 8, 9, 10,];
    for i in 0..10 {
      assert_eq!(i + 1, v[i]);
    }
  }

  #[test]
  fn test_mat_macro_constructor() {
    let m: Matrix<f32, 0, 0> = matrix![];
    assert!(m.is_empty());

    let m = matrix![1];
    assert_eq!(1, m[0][0]);

    let m = matrix![[1, 2], [3, 4], [5, 6],];
    assert_eq!(
      m,
      Matrix::<u32, 3, 2>::from([
        Vector::<u32, 3>::from([1, 3, 5]),
        Vector::<u32, 3>::from([2, 4, 6])
      ])
    );
  }

  #[test]
  fn test_vec_swizzle() {
    let v: Vector<f32, 1> = Vector::<f32, 1>::from([1.0]);
    assert_eq!(1.0, v.x());

    let v: Vector<f32, 2> = Vector::<f32, 2>::from([1.0, 2.0]);
    assert_eq!(1.0, v.x());
    assert_eq!(2.0, v.y());

    let v: Vector<f32, 3> = Vector::<f32, 3>::from([1.0, 2.0, 3.0]);
    assert_eq!(1.0, v.x());
    assert_eq!(2.0, v.y());
    assert_eq!(3.0, v.z());

    let v: Vector<f32, 4> = Vector::<f32, 4>::from([1.0, 2.0, 3.0, 4.0]);
    assert_eq!(1.0, v.x());
    assert_eq!(2.0, v.y());
    assert_eq!(3.0, v.z());
    assert_eq!(4.0, v.w());

    let v: Vector<f32, 5> = Vector::<f32, 5>::from([1.0, 2.0, 3.0, 4.0, 5.0]);
    assert_eq!(1.0, v.x());
    assert_eq!(2.0, v.y());
    assert_eq!(3.0, v.z());
    assert_eq!(4.0, v.w());
  }

  #[test]
  fn test_vec_reflect() {
    // Incident straight on to the surface.
    let v = vector!(1, 0);
    let n = vector!(-1, 0);
    let r = v.reflect(n);
    assert_eq!(r, vector!(-1, 0));

    // Incident at 45 degree angle to the surface.
    let v = vector!(1, 1);
    let n = vector!(-1, 0);
    let r = v.reflect(n);
    assert_eq!(r, vector!(-1, 1));
  }

  #[test]
  fn test_rotation() {
    let rot = Orthonormal::<f32, 3>::from(Euler {
      x: 0.0,
      y: 0.0,
      z: core::f32::consts::FRAC_PI_2,
    });
    assert_eq!(rot.rotate_vector(vector![1.0f32, 0.0, 0.0]).y(), 1.0);
    let v = vector![1.0f32, 0.0, 0.0];
    let q1 = Quaternion::from(Euler {
      x: 0.0,
      y: 0.0,
      z: core::f32::consts::FRAC_PI_2,
    });
    assert_eq!(q1.rotate_vector(v).normalize().y(), 1.0);
  }
}

#[cfg(all(feature = "mint", test))]
mod mint_tests {
  use crate::*;

  #[test]
  fn point2_roundtrip() {
    let alj1 = point![1, 2];
    let mint: mint::Point2<u32> = alj1.into();
    let alj2: Point<u32, 2> = mint.into();
    assert_eq!(alj1, alj2);
  }

  #[test]
  fn point3_roundtrip() {
    let alj1 = point![1, 2, 3];
    let mint: mint::Point3<u32> = alj1.into();
    let alj2: Point<u32, 3> = mint.into();
    assert_eq!(alj1, alj2);
  }

  #[test]
  fn vector2_roundtrip() {
    let alj1 = vector![1, 2];
    let mint: mint::Vector2<u32> = alj1.into();
    let alj2: Vector<u32, 2> = mint.into();
    assert_eq!(alj1, alj2);
  }

  #[test]
  fn vector3_roundtrip() {
    let alj1 = vector![1, 2, 3];
    let mint: mint::Vector3<u32> = alj1.into();
    let alj2: Vector<u32, 3> = mint.into();
    assert_eq!(alj1, alj2);
  }

  #[test]
  fn vector4_roundtrip() {
    let alj1 = vector![1, 2, 3, 4];
    let mint: mint::Vector4<u32> = alj1.into();
    let alj2: Vector<u32, 4> = mint.into();
    assert_eq!(alj1, alj2);
  }

  #[test]
  fn quaternion_roundtrip() {
    let alj1 = Quaternion::new(1, 2, 3, 4);
    let mint: mint::Quaternion<u32> = alj1.into();
    let alj2: Quaternion<u32> = mint.into();
    assert_eq!(alj1, alj2);
  }

  #[test]
  fn matrix2x2_roundtrip() {
    let alj1 = matrix![[1, 2], [3, 4]];
    let mint_col: mint::ColumnMatrix2<u32> = alj1.into();
    let mint_row: mint::RowMatrix2<u32> = alj1.into();
    let alj2: Matrix<u32, 2, 2> = mint_col.into();
    let alj3: Matrix<u32, 2, 2> = mint_row.into();
    assert_eq!(alj1, alj2);
    assert_eq!(alj1, alj3);
  }

  #[test]
  fn matrix3x2_roundtrip() {
    let alj1 = matrix![[1, 2], [3, 4], [5, 6]];
    let mint_col: mint::ColumnMatrix3x2<u32> = alj1.into();
    let mint_row: mint::RowMatrix3x2<u32> = alj1.into();
    let alj2: Matrix<u32, 3, 2> = mint_col.into();
    let alj3: Matrix<u32, 3, 2> = mint_row.into();
    assert_eq!(alj1, alj2);
    assert_eq!(alj1, alj3);
  }

  #[test]
  fn matrix3x4_roundtrip() {
    let alj1 = matrix![[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
    let mint_col: mint::ColumnMatrix3x4<u32> = alj1.into();
    let mint_row: mint::RowMatrix3x4<u32> = alj1.into();
    let alj2: Matrix<u32, 3, 4> = mint_col.into();
    let alj3: Matrix<u32, 3, 4> = mint_row.into();
    assert_eq!(alj1, alj2);
    assert_eq!(alj1, alj3);
  }
}
