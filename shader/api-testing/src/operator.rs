use rendiation_shader_api::*;

use crate::harness::*;

/// all the WGSL matrix multiplications
#[test]
fn matrix_multiplication() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;
    let v2 = f.splat::<Vec2<f32>>();
    let v3 = f.splat::<Vec3<f32>>();
    let v4 = f.splat::<Vec4<f32>>();
    let m2: Node<Mat2<f32>> = (v2, v2).into();
    let m3: Node<Mat3<f32>> = (v3, v3, v3).into();
    let m4: Node<Mat4<f32>> = (v4, v4, v4, v4).into();
    // Mat4x3 can not be composed from columns yet
    let m43 = zeroed_val::<Mat4x3<f32>>();

    // matCxR * vecC -> vecR
    keep(m2 * v2);
    keep(m3 * v3);
    keep(m4 * v4);
    let r: Node<Vec3<f32>> = m43 * v4;
    keep(r);

    // vecR * matCxR -> vecC
    keep(v2 * m2);
    keep(v3 * m3);
    keep(v4 * m4);
    let r: Node<Vec4<f32>> = v3 * m43;
    keep(r);

    // matKxR * matCxK -> matCxR
    keep(m4 * m4);
    keep(m43 * m4);
    keep(m3 * m43);

    // matrix and scalar
    keep(m4 * f);
    keep(f * m4);
    keep(m4 + m4 - m4);
  });
}

/// component-wise arithmetic
#[test]
fn component_wise_arithmetic() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    let v3 = f.splat::<Vec3<f32>>();
    let iv3 = i.splat::<Vec3<i32>>();
    let uv3 = u.splat::<Vec3<u32>>();

    keep(v3 + v3 - v3 * v3 / v3 % v3);
    keep(iv3 + iv3 - iv3 * iv3 / iv3 % iv3);
    keep(uv3 + uv3 - uv3 * uv3 / uv3 % uv3);
    keep(v3 * f + f * v3);
    keep(-v3);
    keep(-i);
    keep(-iv3);
  });
}

/// vector and scalar mixed arithmetic in both order, and the compound assignments
#[test]
fn vector_scalar_mixed_arithmetic() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);

    macro_rules! mixed {
      ($s: expr, $($vec: ty),+) => {
        $(
          let s = $s;
          let v = s.splat::<$vec>();
          keep(v + s);
          keep(s + v);
          keep(v - s);
          keep(s - v);
          keep(v * s);
          keep(s * v);
          keep(v / s);
          keep(s / v);
          keep(v % s);
          keep(s % v);

          let mut a = v;
          a += s;
          a -= s;
          a *= s;
          a /= s;
          a += v;
          keep(a);
        )+
      };
    }

    mixed!(f, Vec2<f32>, Vec3<f32>, Vec4<f32>);
    mixed!(i, Vec2<i32>, Vec3<i32>, Vec4<i32>);
    mixed!(u, Vec2<u32>, Vec3<u32>, Vec4<u32>);

    let v3 = f.splat::<Vec3<f32>>();
    let m3: Node<Mat3<f32>> = (v3, v3, v3).into();
    let mut a = v3;
    a *= m3;
    keep(a);
    let mut m = m3;
    m += m3;
    m *= f;
    m *= m3;
    keep(m);
  });
}

/// logical and bitwise operators
#[test]
fn logical_and_bitwise() {
  check_compute(|builder| {
    let RuntimeValues { u, i, .. } = runtime_values(builder);
    let b = u.equals(0);
    let bv2: Node<Vec2<bool>> = (b, b.not()).into();

    keep(b & b.not());
    keep(b | b.not());
    keep(bv2 & bv2);
    keep(bv2.not());
    keep(b.and(b).or(b));
    keep(u & u | u ^ u);
    keep(i & i | i ^ i);
    keep(u.bitwise_not() << u >> u);
  });
}

/// conversion and construction
#[test]
fn conversion_and_construction() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    keep(f.into_bool().into_f32());
    keep(i.into_bool().into_i32());
    keep(u.into_bool().into_u32());
    keep(u.splat::<Vec4<u32>>());
    keep(u.equals(0).splat::<Vec3<bool>>());
    let bv: Node<Vec3<bool>> = (u.equals(0), u.equals(1), u.equals(2)).into();
    keep(bv);
  });
}
