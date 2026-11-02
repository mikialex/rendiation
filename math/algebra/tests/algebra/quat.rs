use std::f64::consts::*;

use glam::DQuat;
use rendiation_algebra::*;

use crate::util::*;

#[test]
fn construction() {
  for_each_seed(|r| {
    let (angle, axis) = (r.angle(), r.unit_vec3());
    assert_near(Quat::rotation_x(angle), DQuat::from_rotation_x(angle));
    assert_near(Quat::rotation_y(angle), DQuat::from_rotation_y(angle));
    assert_near(Quat::rotation_z(angle), DQuat::from_rotation_z(angle));
    assert_near(
      Quat::rotation(axis, angle),
      DQuat::from_axis_angle(axis.to_glam(), angle),
    );
  });
  assert_near(Quat::<f64>::identity(), DQuat::IDENTITY);
  assert_eq!(Quat::<f64>::default(), Quat::new(0., 0., 0., 0.));
}

#[test]
fn algebra() {
  for_each_seed(|r| {
    let (a, b, s) = (r.quat(), r.quat(), r.range(0.5, 2.));
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a * b, ga * gb);
    assert_near(a + b, ga + gb);
    assert_near(a - b, ga - gb);
    assert_near(-a, -ga);
    assert_near(a * s, ga * s);
    assert_near(a / s, ga / s);
    assert_near(a.dot(b), ga.dot(gb));
    assert_near((a * s).length(), (ga * s).length());
    assert_near((a * s).length2(), (ga * s).length_squared());
    assert_near((a * s).normalize(), ga);
    assert_near(a.distance(b), (ga - gb).length());
    assert_near(a.conjugate(), ga.conjugate());
    assert_near(a.inverse(), ga.inverse());
    assert_near_eps(a * a.inverse(), Quat::identity(), 1e-12);
  });

  assert_eq!(Quat::<f64>::default().normalize(), Quat::identity());
}

#[test]
fn rotate_vector() {
  for_each_seed(|r| {
    let (q, v) = (r.quat(), r.vec3());
    let expect = q.to_glam() * v.to_glam();
    assert_near(q.rotate_vec(v), expect);
    assert_near(Mat3::from(q) * v, expect);
    // the multiplication applies the right side first
    let q2 = r.quat();
    assert_near((q * q2).rotate_vec(v), q.rotate_vec(q2.rotate_vec(v)));
  });
}

#[test]
fn axis_angle() {
  for_each_seed(|r| {
    let (axis, angle) = (r.unit_vec3(), r.range(0.01, 2. * PI - 0.01));
    let q = Quat::rotation(axis, angle);
    let (g_axis, g_angle) = q.to_glam().to_axis_angle();
    assert_near_eps(q.axis(), g_axis, 1e-7);
    assert_near_eps(q.angle(), g_angle, 1e-7);
    assert_near_eps(q.axis(), axis, 1e-7);
    assert_near_eps(q.angle(), angle, 1e-7);
  });

  let identity = Quat::<f64>::identity();
  assert_eq!(identity.axis(), Vec3::new(1., 0., 0.));
  assert_eq!(identity.angle(), 0.);
  // precision error should not produce nan
  let q = Quat::new(0., 0., 0., 1. + 1e-12);
  assert_eq!(q.angle(), 0.);
}

#[test]
fn from_mat3() {
  for_each_seed(|r| {
    let q = r.quat();
    let m = Mat3::from(q);
    let back = Quat::from(m);
    assert_same_rotation(back, q.to_glam());
    assert_same_rotation(back, DQuat::from_mat3(&m.to_glam()));
  });

  // cover all the branches: the trace is positive, or the max diagonal element is x, y, z
  let cases = [
    Quat::identity(),
    Quat::rotation_x(PI),
    Quat::rotation_y(PI),
    Quat::rotation_z(PI),
    Quat::rotation_x(PI * 0.9),
    Quat::rotation_y(-PI * 0.9),
    Quat::rotation_z(PI * 0.95),
    Quat::rotation(Vec3::new(1., 1., 1.).normalize(), PI),
  ];
  for q in cases {
    let back = Quat::from(Mat3::from(q));
    assert_same_rotation(back, q.to_glam());
    assert_near_eps(back.length(), 1., 1e-9);
  }
}

#[test]
fn direction() {
  for_each_seed(|r| {
    let (a, b) = (r.unit_vec3(), r.unit_vec3());
    let q = Quat::direction(&a, &b);
    assert_same_rotation(q, DQuat::from_rotation_arc(a.to_glam(), b.to_glam()));
    assert_near_eps(q.rotate_vec(a), b, 1e-7);
    assert_near_eps(q.length(), 1., 1e-9);

    // the opposite direction rotates 180 degree
    let q = Quat::direction(&a, &-a);
    assert!(!q.x.is_nan());
    assert_near_eps(q.length(), 1., 1e-9);
    assert_near_eps(q.rotate_vec(a), -a, 1e-7);
  });

  let x = Vec3::new(1., 0., 0.);
  let q = Quat::direction(&x, &x);
  assert_near(q, Quat::identity());
  for axis in [x, Vec3::new(0., 1., 0.), Vec3::new(0., 0., 1.)] {
    let q = Quat::direction(&axis, &-axis);
    assert_near(q.rotate_vec(axis), -axis);
  }
}

#[test]
fn slerp() {
  for_each_seed(|r| {
    let (a, b, t) = (r.quat(), r.quat(), r.range(0., 1.));
    let result = a.slerp(b, t);
    assert_same_rotation(result, a.to_glam().slerp(b.to_glam(), t));
    assert_near_eps(result.length(), 1., 1e-9);

    // the shortest path is taken
    let result = a.slerp(-b, t);
    assert_same_rotation(result, a.to_glam().slerp(b.to_glam(), t));
  });

  let a = Quat::rotation_z(0.0);
  let b = Quat::rotation_z(1.0);
  assert_eq!(a.slerp(b, 0.), a);
  assert_eq!(a.slerp(b, 1.), b);
  assert_near(a.slerp(b, 0.5), Quat::rotation_z(0.5));
  // nearly identical input falls back to lerp
  let c = Quat::rotation_z(1e-9);
  assert_near(a.slerp(c, 0.5), Quat::rotation_z(0.5e-9));
  assert_eq!(a.slerp(a, 0.3), a);
}

#[test]
fn conversion() {
  let q = Quat::new(1., 2., 3., 4.);
  let arr: [f64; 4] = q.into();
  assert_eq!(arr, [1., 2., 3., 4.]);
  assert_eq!(Quat::from(arr), q);
  let t: (f64, f64, f64, f64) = q.into();
  assert_eq!(t, (1., 2., 3., 4.));
  assert_eq!(Quat::from(t), q);
  let v: Vec4<f64> = q.into();
  assert_eq!(v, Vec4::new(1., 2., 3., 4.));
  assert_eq!(Quat::from(v), q);
  assert_eq!(q.into_f32().into_f64(), q);
  assert_eq!(q.map(|v| v as i32), Quat::new(1, 2, 3, 4));
  // same memory layout as glam
  let q = q.into_f32();
  let raw: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&q));
  assert_eq!(raw, glam::Quat::from_xyzw(1., 2., 3., 4.).to_array());
}
