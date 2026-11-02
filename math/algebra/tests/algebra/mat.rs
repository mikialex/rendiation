use std::f64::consts::*;

use glam::{DAffine3, DMat2, DMat3, DMat4, DVec2, DVec3, DVec4};
use rendiation_algebra::*;

use crate::util::*;

#[test]
fn layout_and_constructor() {
  // the constructor arguments are column by column, same as glam's from_cols_array
  #[rustfmt::skip]
  let m = Mat4::new(
    1., 2., 3., 4.,
    5., 6., 7., 8.,
    9., 10., 11., 12.,
    13., 14., 15., 16.,
  );
  let arr: [f64; 16] = m.into();
  assert_eq!(arr, DMat4::from_cols_array(&arr).to_cols_array());
  assert_eq!(m.to_glam().col(1), DVec4::new(5., 6., 7., 8.));
  assert_eq!(
    Mat4::new_from_column(
      [1., 2., 3., 4.],
      [5., 6., 7., 8.],
      [9., 10., 11., 12.],
      [13., 14., 15., 16.]
    ),
    m
  );
  assert_eq!(m.right(), Vec3::new(1., 2., 3.));
  assert_eq!(m.up(), Vec3::new(5., 6., 7.));
  assert_eq!(m.forward(), Vec3::new(9., 10., 11.));
  assert_eq!(m.position(), Vec3::new(13., 14., 15.));
  assert_eq!(m.remove_position().position(), Vec3::zero());
  assert_eq!(m.remove_position().d4, 16.);

  let m3 = Mat3::new(1., 2., 3., 4., 5., 6., 7., 8., 9.);
  assert_eq!(m3.to_glam().col(2), DVec3::new(7., 8., 9.));
  assert_eq!(m3.right(), Vec3::new(1., 2., 3.));
  assert_eq!(m3.up(), Vec3::new(4., 5., 6.));
  assert_eq!(m3.forward(), Vec3::new(7., 8., 9.));
  assert_eq!(m3.to_mat2(), Mat2::new(1., 2., 4., 5.));
  assert_eq!(m.to_mat3(), Mat3::new(1., 2., 3., 5., 6., 7., 9., 10., 11.));

  let m2 = Mat2::new(1., 2., 3., 4.);
  assert_eq!(m2.to_glam().col(1), DVec2::new(3., 4.));
  assert_eq!(m2.right(), Vec2::new(1., 2.));
  assert_eq!(m2.up(), Vec2::new(3., 4.));
  assert_eq!(Mat2::from((1., 2., 3., 4.)), m2);

  assert_eq!(matrix!(1., 2., 3., 4.), m2);
  assert_eq!(matrix!(1., 2., 3., 4., 5., 6., 7., 8., 9.), m3);
  #[rustfmt::skip]
  let macro_m = matrix!(
    1., 2., 3., 4.,
    5., 6., 7., 8.,
    9., 10., 11., 12.,
    13., 14., 15., 16.
  );
  assert_eq!(macro_m, m);

  let m_f32 = m.into_f32();
  assert_eq!(m_f32.into_f64(), m);
  assert_eq!(m_f32.map(|v| v as i32).d4, 16);
  let raw: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&m_f32));
  assert_eq!(
    raw,
    &glam::Mat4::from_cols_array(&m_f32.into()).to_cols_array()
  );
  assert_eq!(std::mem::size_of::<Mat3<f32>>(), 36);
}

#[test]
fn identity_zero() {
  assert_near(Mat2::<f64>::identity(), DMat2::IDENTITY);
  assert_near(Mat3::<f64>::identity(), DMat3::IDENTITY);
  assert_near(Mat4::<f64>::identity(), DMat4::IDENTITY);
  assert_near(Mat4::<f64>::zero(), DMat4::ZERO);
  assert!(Mat3::<f64>::zero().is_zero());
  assert!(!Mat3::<f64>::identity().is_zero());
  assert_eq!(Mat4::<f64>::one(), Mat4::identity());
}

#[test]
fn component_wise_ops() {
  for_each_seed(|r| {
    let (a, b, s) = (r.mat4(), r.mat4(), r.range(0.5, 2.));
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a + b, ga + gb);
    assert_near(a - b, ga - gb);
    assert_near(-a, -ga);
    assert_near(a * s, ga * s);
    assert_near(s * a, ga * s);
    assert_near(a / s, ga / s);

    let mut c = a;
    c += b;
    assert_near(c, ga + gb);
    c -= b;
    assert_near(c, ga);
    c *= s;
    assert_near(c, ga * s);
    c /= s;
    assert_near(c, ga);

    let (a, b) = (r.mat3(), r.mat3());
    assert_near(a + b, a.to_glam() + b.to_glam());
    assert_near(a * s - b, a.to_glam() * s - b.to_glam());
    let (a, b) = (r.mat2(), r.mat2());
    assert_near(a + b, a.to_glam() + b.to_glam());
    assert_near(-a / s, -a.to_glam() / s);
  });
}

#[test]
fn mat2() {
  for_each_seed(|r| {
    let (a, b, v) = (r.mat2(), r.mat2(), r.vec2());
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a * b, ga * gb);
    assert_near(a * v, ga * v.to_glam());
    assert_near(a.transpose(), ga.transpose());
    assert_near(a.det(), ga.determinant());
    assert_near_eps(a.inverse().unwrap(), ga.inverse(), 1e-7);
    assert_near_eps(a * a.inverse().unwrap(), Mat2::identity(), 1e-7);

    let angle = r.angle();
    assert_near(Mat2::rotate_z(angle), DMat2::from_angle(angle));
    assert_near(
      Mat2::rotate(Vec3::new(0., 0., 1.), angle),
      Mat2::rotate_z(angle),
    );
    let axis = r.unit_vec3();
    assert_near(
      Mat2::rotate(axis, angle),
      Mat3::rotate(axis, angle).to_mat2(),
    );
    assert_near(Mat2::rotate_x(angle), Mat3::rotate_x(angle).to_mat2());
    assert_near(Mat2::rotate_y(angle), Mat3::rotate_y(angle).to_mat2());

    let (sx, sy) = (r.f(), r.f());
    assert_near(
      Mat2::scale(sx, sy),
      DMat2::from_diagonal(DVec2::new(sx, sy)),
    );
    let scaled_rotation = Mat2::rotate_z(angle) * Mat2::scale(2., 3.);
    assert_near(scaled_rotation.max_scale(), 3.);
  });

  assert_eq!(Mat2::new(1., 2., 2., 4.).inverse(), None);
  assert_eq!(
    Mat2::new(1., 2., 2., 4.).inverse_or_identity(),
    Mat2::identity()
  );
}

#[test]
fn mat3() {
  for_each_seed(|r| {
    let (a, b, v) = (r.mat3(), r.mat3(), r.vec3());
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a * b, ga * gb);
    assert_near(a * v, ga * v.to_glam());
    assert_near(a.transpose(), ga.transpose());
    assert_near(a.det(), ga.determinant());
    assert_near_eps(a.inverse().unwrap(), ga.inverse(), 1e-6);
    assert_near_eps(a * a.inverse().unwrap(), Mat3::identity(), 1e-6);

    let angle = r.angle();
    let axis = r.unit_vec3();
    assert_near(Mat3::rotate_x(angle), DMat3::from_rotation_x(angle));
    assert_near(Mat3::rotate_y(angle), DMat3::from_rotation_y(angle));
    assert_near(Mat3::rotate_z(angle), DMat3::from_rotation_z(angle));
    assert_near(
      Mat3::rotate(axis, angle),
      DMat3::from_axis_angle(axis.to_glam(), angle),
    );

    let (t, s) = (r.vec2(), r.vec2());
    assert_near(Mat3::translate(t), DMat3::from_translation(t.to_glam()));
    assert_near(Mat3::scale(s), DMat3::from_scale(s.to_glam()));
    // the rotate_z is the 2d rotation in homogeneous 2d space
    assert_near(Mat3::rotate_z(angle), DMat3::from_angle(angle));

    let m = Mat3::translate(t) * Mat3::rotate_z(angle) * Mat3::scale(s);
    let gm = DMat3::from_scale_angle_translation(s.to_glam(), angle, t.to_glam());
    assert_near(m, gm);
    let p = r.vec2();
    assert_near(m * p, gm.transform_point2(p.to_glam()));

    let q = r.quat();
    assert_near(Mat3::from(q), DMat3::from_quat(q.to_glam()));

    let expect = gm.inverse().transpose();
    let expect = DMat2::from_cols(expect.x_axis.truncate(), expect.y_axis.truncate());
    assert_near_eps(m.to_normal_matrix(), expect, 1e-7);

    // the 2d max scale
    let m = Mat3::rotate_z(angle) * Mat3::scale((0.5, 1.5));
    assert_near(m.max_scale(), 1.5);
  });

  assert_eq!(Mat3::<f64>::zero().inverse(), None);
  let projective = Mat3::new(1., 0., 1., 0., 1., 0., 0., 0., 1.);
  assert_near(projective * Vec2::new(1., 2.), Vec2::new(0.5, 1.));
}

#[test]
fn mat4() {
  for_each_seed(|r| {
    let (a, b, v) = (r.mat4(), r.mat4(), r.vec4());
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a * b, ga * gb);
    assert_near(a * v, ga * v.to_glam());
    assert_near(a.transpose(), ga.transpose());
    assert_near(a.det(), ga.determinant());
    assert_near_eps(a.inverse().unwrap(), ga.inverse(), 1e-6);
    assert_near_eps(a * a.inverse().unwrap(), Mat4::identity(), 1e-6);
    let p = r.vec3();
    assert_near_eps(a * p, ga.project_point3(p.to_glam()), 1e-7);

    let angle = r.angle();
    let axis = r.unit_vec3();
    assert_near(Mat4::rotate_x(angle), DMat4::from_rotation_x(angle));
    assert_near(Mat4::rotate_y(angle), DMat4::from_rotation_y(angle));
    assert_near(Mat4::rotate_z(angle), DMat4::from_rotation_z(angle));
    assert_near(
      Mat4::rotate(axis, angle),
      DMat4::from_axis_angle(axis.to_glam(), angle),
    );

    let (t, s) = (r.vec3(), r.scale3());
    assert_near(Mat4::translate(t), DMat4::from_translation(t.to_glam()));
    assert_near(Mat4::scale(s), DMat4::from_scale(s.to_glam()));

    let q = r.quat();
    assert_near(Mat4::from(q), DMat4::from_quat(q.to_glam()));
    assert_near(Mat4::from(Mat3::from(q)), DMat4::from_quat(q.to_glam()));
    let gm = DMat4::from_scale_rotation_translation(s.to_glam(), q.to_glam(), t.to_glam());
    let m = Mat4::compose(t, q, s);
    assert_near(m, gm);
    assert_near(m.get_scale(), s);
    assert_near_eps(
      m.extract_rotation_mat(),
      DMat4::from_quat(q.to_glam()),
      1e-7,
    );
    assert_near_eps(m.transform_inverse().unwrap(), gm.inverse(), 1e-7);
    assert_near(m.max_scale(), s.max_channel());

    let normal = DMat3::from_mat4(gm).inverse().transpose();
    assert_near_eps(m.to_normal_matrix(), normal, 1e-7);
  });

  assert_eq!(Mat4::<f64>::zero().inverse(), None);
  assert_eq!(Mat4::<f64>::zero().transform_inverse(), None);
}

#[test]
fn decompose() {
  for_each_seed(|r| {
    let m = r.trs();
    let (t, q, s) = m.decompose();
    let (gs, gq, gt) = m.to_glam().to_scale_rotation_translation();
    assert_near_eps(t, gt, 1e-7);
    assert_near_eps(s, gs, 1e-7);
    assert_same_rotation(q, gq);
    assert_near_eps(Mat4::compose(t, q, s), m, 1e-7);

    // the mirror transform: the negative scale is put into x
    let mirror = m * Mat4::scale((1., -1., 1.));
    let (t, q, s) = mirror.decompose();
    let (gs, gq, gt) = mirror.to_glam().to_scale_rotation_translation();
    assert!(s.x < 0.);
    assert_near_eps(t, gt, 1e-7);
    assert_near_eps(s, gs, 1e-7);
    assert_same_rotation(q, gq);
    assert_near_eps(Mat4::compose(t, q, s), mirror, 1e-7);
    assert_near_eps(mirror.extract_rotation_mat().det(), 1., 1e-7);
  });
}

#[test]
fn lookat() {
  for_each_seed(|r| {
    let (eye, center) = (r.vec3(), r.vec3());
    let up = Vec3::new(0., 1., 0.);
    let m = Mat4::lookat(eye, center, up);
    // the lookat is the camera world matrix, which is the inverse of the view matrix
    let view = glam::dcamera::rh::view::look_at_mat4(eye.to_glam(), center.to_glam(), up.to_glam());
    assert_near_eps(m.inverse().unwrap(), view, 1e-7);
    assert_near_eps(
      m * Vec3::new(0., 0., -(eye - center).length()),
      center,
      1e-7,
    );
    assert_near(m.position(), eye);
  });
}

#[test]
fn space_dimension() {
  fn dimension<const D: usize, M: SquareMatrixDimension<D>>(_: M) -> usize {
    D
  }
  assert_eq!(dimension(Mat2::<f32>::identity()), 1);
  assert_eq!(dimension(Mat3::<f32>::identity()), 2);
  assert_eq!(dimension(Mat4::<f32>::identity()), 3);
}

#[test]
fn mat4x3() {
  for_each_seed(|r| {
    let m = r.trs();
    let affine: Mat4x3<f64> = m.into();
    let ga = DAffine3::from_mat4(m.to_glam());

    // round trip keeps the translation
    assert_eq!(Mat4::from(affine), m);
    assert_eq!(affine.position(), m.position());
    assert_eq!(affine.to_mat3(), m.to_mat3());
    assert_eq!(affine.right(), m.right());
    assert_eq!(affine.up(), m.up());
    assert_eq!(affine.forward(), m.forward());
    assert_near(affine.get_scale(), m.get_scale());
    assert_eq!(affine.to_normal_matrix(), m.to_normal_matrix());

    let p = r.vec3();
    assert_near(affine * p, ga.transform_point3(p.to_glam()));
    let v = r.vec4();
    assert_near(affine * v, (m * v).xyz());

    let rot = Mat3::from(r.quat());
    let affine = Mat4x3::from(rot);
    assert_eq!(Mat4::from(affine), Mat4::from(rot));

    let (forward, up, position) = (Vec3::new(0., 0., 1.), Vec3::new(0., 1., 0.), r.vec3());
    let a = Mat4x3::from_orth_basis_and_position(forward, up, position);
    let b = Mat4::from_orth_basis_and_position(forward, up, position);
    assert_eq!(Mat4::from(a), b);
    assert_eq!(a.right(), Vec3::new(-1., 0., 0.));
  });

  #[rustfmt::skip]
  let m = Mat4x3::new(
    1., 2., 3.,
    4., 5., 6.,
    7., 8., 9.,
    10., 11., 12.,
  );
  assert_eq!(
    Mat4x3::new_from_column([1., 2., 3.], [4., 5., 6.], [7., 8., 9.], [10., 11., 12.]),
    m
  );
  let arr: [f64; 12] = m.into();
  assert_eq!(arr, [1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12.]);
  assert_eq!(m * 2., Mat4x3::from(arr.map(|v| v * 2.)));
}

#[test]
fn non_square() {
  let m = Mat2x3::new_from_column([1, 2, 3], [4, 5, 6]);
  let arr: [i32; 6] = m.into();
  assert_eq!(arr, [1, 2, 3, 4, 5, 6]);
  assert_eq!(m + m, Mat2x3::from(arr.map(|v| v * 2)));

  let m = Mat2x4::new_from_column([1, 2, 3, 4], [5, 6, 7, 8]);
  assert_eq!((m * 2).b4, 16);
  let m = Mat3x2::new_from_column([1, 2], [3, 4], [5, 6]);
  assert_eq!((-m).c2, -6);
  let m = Mat3x4::new_from_column([1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]);
  assert_eq!((m - m).c4, 0);
  let m = Mat4x2::new_from_column([1, 2], [3, 4], [5, 6], [7, 8]);
  assert_eq!((m / 2).d2, 4);

  assert_eq!(std::mem::size_of::<Mat4x3<f32>>(), 48);
  assert_eq!(std::mem::size_of::<Mat2x3<f32>>(), 24);
}

#[test]
fn rotation_handedness() {
  // positive angle rotates counter clockwise looking from the positive axis
  let r = Mat4::rotate_z(FRAC_PI_2);
  assert_near(r * Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.));
  let r = Mat4::rotate_x(FRAC_PI_2);
  assert_near(r * Vec3::new(0., 1., 0.), Vec3::new(0., 0., 1.));
  let r = Mat4::rotate_y(FRAC_PI_2);
  assert_near(r * Vec3::new(0., 0., 1.), Vec3::new(1., 0., 0.));
  let r = Mat2::rotate_z(FRAC_PI_2);
  assert_near(r * Vec2::new(1., 0.), Vec2::new(0., 1.));
  // matrix multiplication applies the right side first
  let m = Mat4::translate((1., 0., 0.)) * Mat4::rotate_z(FRAC_PI_2);
  assert_near(m * Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.));
}
