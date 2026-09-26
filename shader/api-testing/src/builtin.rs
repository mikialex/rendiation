use rendiation_shader_api::*;

use crate::harness::*;

/// float built-in functions
#[test]
fn float_functions() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;
    let v3 = f.splat::<Vec3<f32>>();

    keep(f.saturate());
    keep(v3.saturate());
    keep(f.smoothstep(0.0, 1.0));
    keep(v3.smoothstep_per_channel(v3, v3));
    keep(f.mix(v3, v3));
    keep(f.degrees() + f.radians());
    keep(f.fma(f, f));
    keep(v3.face_forward(v3, v3));
    keep(v3.length());
    keep(v3.quantize_to_f16());
    keep(f.is_nan().or(f.is_inf()));
    keep(f.step(val(0.5)));
  });
}

/// the built-in functions that return struct
#[test]
fn decomposition_functions() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;
    let v3 = f.splat::<Vec3<f32>>();

    let (fract, exp): (_, Node<Vec3<i32>>) = v3.frexp();
    keep(fract);
    keep(exp);
    let (fract, exp): (_, Node<i32>) = f.frexp();
    keep(fract);
    keep(exp);
    let (fract, whole) = v3.modf();
    keep(fract + whole);
    keep(v3.ldexp(exp.splat::<Vec3<i32>>()));
  });
}

/// matrix and vector functions
#[test]
fn matrix_and_vector_functions() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    let v4 = f.splat::<Vec4<f32>>();
    let m4: Node<Mat4<f32>> = (v4, v4, v4, v4).into();

    keep(m4.determinant());
    keep(m4.transpose());
    keep(u.splat::<Vec3<u32>>().dot(u.splat::<Vec3<u32>>()));
    keep(i.splat::<Vec3<i32>>().dot(i.splat::<Vec3<i32>>()));
    keep(u.dot4_u8_packed(u));
    keep(u.dot4_i8_packed(u));
  });
}

/// integer bit functions
#[test]
fn bit_functions() {
  check_compute(|builder| {
    let RuntimeValues { u, i, .. } = runtime_values(builder);
    keep(u.count_leading_zeros() + u.count_trailing_zeros() + u.count_one_bits());
    keep(u.reverse_bits() + u.first_leading_bit() + u.first_trailing_bit());
    keep(i.first_leading_bit() + i.first_trailing_bit());
    keep(
      u.extract_bits(val(1), val(2))
        .insert_bits(u, val(1), val(2)),
    );
  });
}

/// packing and unpacking
#[test]
fn packing() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    keep(i.splat::<Vec4<i32>>().pack4x_i8() + i.splat::<Vec4<i32>>().pack4x_i8_clamp());
    keep(u.splat::<Vec4<u32>>().pack4x_u8() + u.splat::<Vec4<u32>>().pack4x_u8_clamp());
    keep(f.splat::<Vec4<f32>>().pack4x8unorm() + f.splat::<Vec2<f32>>().pack2x16float());
    keep(u.unpack4x_i8());
    keep(u.unpack4x_u8());
    keep(u.unpack4x8snorm());
  });
}

/// bitcast between the 32 bit numeric scalars or vectors of the same shape
#[test]
fn bitcast() {
  check_compute(|builder| {
    let RuntimeValues { u, i, f } = runtime_values(builder);
    keep(f.bitcast::<u32>() + u.bitcast::<f32>().bitcast::<u32>());
    keep(i.bitcast::<f32>().bitcast::<f32>());
    keep(u.splat::<Vec2<u32>>().bitcast::<Vec2<i32>>());
    keep(f.splat::<Vec3<f32>>().bitcast::<Vec3<u32>>());
    keep(i.splat::<Vec4<i32>>().bitcast::<Vec4<f32>>());
  });
}
