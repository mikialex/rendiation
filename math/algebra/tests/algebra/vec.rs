use glam::{DVec2, DVec3, Vec3A};
use rendiation_algebra::*;

use crate::util::*;

#[test]
fn arithmetic_ops() {
  for_each_seed(|r| {
    let (a, b, s) = (r.vec3(), r.vec3(), r.range(0.5, 2.));
    let (ga, gb) = (a.to_glam(), b.to_glam());

    assert_near(a + b, ga + gb);
    assert_near(a - b, ga - gb);
    assert_near(a * b, ga * gb);
    assert_near(a / b, ga / gb);
    assert_near(a * s, ga * s);
    assert_near(s * a, s * ga);
    assert_near(a / s, ga / s);
    assert_near(s / a, s / ga);
    assert_near(-a, -ga);
    assert_near(a % s, ga % s);

    let mut c = a;
    c += b;
    assert_near(c, ga + gb);
    c -= b;
    assert_near(c, ga);
    c *= s;
    assert_near(c, ga * s);
    c /= s;
    assert_near(c, ga);
    c %= s;
    assert_near(c, ga % s);

    let (a, b) = (r.vec4(), r.vec4());
    assert_near(a + b, a.to_glam() + b.to_glam());
    assert_near(-a * s, -a.to_glam() * s);
    let (a, b) = (r.vec2(), r.vec2());
    assert_near(a - b, a.to_glam() - b.to_glam());
    assert_near(a / s, a.to_glam() / s);
  });

  // integer vectors share the same operator implementation
  let a = Vec2::new(3_u32, 7);
  assert_eq!(a * 2, Vec2::new(6, 14));
  assert_eq!(2 * a, Vec2::new(6, 14));
  assert_eq!(a / 2, Vec2::new(1, 3));
  assert_eq!(a % 3, Vec2::new(0, 1));
  assert_eq!(-Vec3::new(1_i32, -2, 3), Vec3::new(-1, 2, -3));
}

#[test]
fn inner_product() {
  for_each_seed(|r| {
    let (a, b) = (r.vec3(), r.vec3());
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a.dot(b), ga.dot(gb));
    assert_near(a.length(), ga.length());
    assert_near(a.length2(), ga.length_squared());
    assert_near(a.distance_to(b), ga.distance(gb));
    assert_near(a.distance2_to(b), ga.distance_squared(gb));
    assert_near(a.normalize(), ga.normalize());
    assert_near(a.cross(b), ga.cross(gb));
    assert_near(a.reverse(), -ga);

    let n = r.unit_vec3();
    assert_near(a.reflect(n), ga.reflect(n.to_glam()));
    assert_near(a.reflect(n.into_normalized()), ga.reflect(n.to_glam()));
    assert_near(a.dot(n.into_normalized()), ga.dot(n.to_glam()));

    let mut c = a;
    let len = c.normalize_self();
    assert_near(len, ga.length());
    assert_near(c, ga.normalize());

    let (a, b) = (r.vec4(), r.vec4());
    assert_near(a.dot(b), a.to_glam().dot(b.to_glam()));
    assert_near(a.normalize(), a.to_glam().normalize());

    let (a, b) = (r.vec2(), r.vec2());
    assert_near(a.dot(b), a.to_glam().dot(b.to_glam()));
    assert_near(a.cross(b), a.to_glam().perp_dot(b.to_glam()));
    assert_near(a.normalize(), a.to_glam().normalize());
  });

  // the zero vector is not changed by normalization
  assert_eq!(Vec3::<f32>::zero().normalize(), Vec3::zero());
  let mut z = Vec2::<f32>::zero();
  assert_eq!(z.normalize_self(), 0.);
  assert_eq!(z, Vec2::zero());
}

#[test]
fn component_wise() {
  for_each_seed(|r| {
    let (a, b, t) = (r.vec3(), r.vec3(), r.f());
    let (ga, gb) = (a.to_glam(), b.to_glam());
    assert_near(a.lerp(b, t), ga.lerp(gb, t));
    assert_near(RealVector::min(a, b), ga.min(gb));
    assert_near(RealVector::max(a, b), ga.max(gb));
    let (lo, hi) = (ga.min(gb), ga.max(gb));
    let c = r.vec3();
    assert_near(
      c.clamp(RealVector::min(a, b), RealVector::max(a, b)),
      c.to_glam().clamp(lo, hi),
    );
    assert_near(
      (a / 10.).saturate(),
      (ga / 10.).clamp(DVec3::ZERO, DVec3::ONE),
    );
    assert_near(a.max_channel(), ga.max_element());
    assert_near(a.min_channel(), ga.min_element());
    assert_eq!(a.max_channel_index(), ga.max_position());
    assert_near(a.map(|v| v * 2.), ga * 2.);
    assert_near(a.zip(b, |x, y| x * y + 1.), ga * gb + 1.);
  });

  assert_eq!(Vec3::new(1., 1., 0.).max_channel_index(), 0);
  assert_eq!(Vec3::new(0., 1., 1.).max_channel_index(), 1);
  assert_eq!(Vec3::new(0., 1., 2.).max_channel_index(), 2);
}

#[test]
fn vector_trait() {
  assert_eq!(<Vec3<f32> as Vector<f32>>::one(), Vec3::new(1., 1., 1.));
  assert_eq!(
    <Vec4<f32> as Vector<f32>>::zero(),
    Vec4::new(0., 0., 0., 0.)
  );
  assert_eq!(Vec2::splat(3.), Vec2::new(3., 3.));
  assert_eq!(Vec2::<f32>::channel_count(), 2);
  assert_eq!(Vec3::<f32>::channel_count(), 3);
  assert_eq!(Vec4::<f32>::channel_count(), 4);
  assert_eq!(Vec4::<f32>::len(), 4);
  assert_eq!(Vec4::<u8>::create(|| 7), Vec4::new(7, 7, 7, 7));
}

#[test]
fn vec2_rotation() {
  for_each_seed(|r| {
    let (v, anchor, angle) = (r.vec2(), r.vec2(), r.angle());
    let expect = anchor.to_glam() + DVec2::from_angle(angle).rotate(v.to_glam() - anchor.to_glam());
    assert_near(v.rotate(anchor, angle), expect);
    assert_near(
      v.rotate(anchor, angle),
      Mat2::rotate_z(angle) * (v - anchor) + anchor,
    );

    assert_near(v.perpendicular_ccw(), v.to_glam().perp());
    assert_near(v.perpendicular_cw(), -v.to_glam().perp());
  });

  // rotating the anchor itself is a no op
  let anchor = Vec2::new(3., 4.);
  assert_near(anchor.rotate(anchor, 1.), anchor);
  let v = Vec2::new(1., 0.).rotate(Vec2::zero(), std::f64::consts::FRAC_PI_2);
  assert_near(v, Vec2::new(0., 1.));
}

#[test]
fn vec3_transform() {
  for_each_seed(|r| {
    let (m, v) = (r.trs(), r.vec3());
    let gm = m.to_glam();
    let expect = gm.transform_vector3(v.to_glam()).normalize();
    assert_near(*v.transform_direction(m), expect);

    assert_near(m * v, gm.project_point3(v.to_glam()));
    assert_near(m * v, gm.transform_point3(v.to_glam()));
    assert_near(*v.clone().apply_matrix(m), gm.transform_point3(v.to_glam()));
    assert_near(
      SpaceEntityCopyable::apply_matrix_into(&v, m),
      gm.transform_point3(v.to_glam()),
    );

    let m3 = Mat3::translate(r.vec2()) * Mat3::scale(r.vec2());
    let v2 = r.vec2();
    assert_near(m3 * v2, m3.to_glam().transform_point2(v2.to_glam()));
    assert_near(*v2.clone().apply_matrix(m3), m3 * v2);
  });
}

#[test]
fn local_to_world() {
  let mut dirs: Vec<_> = (0..64).map(|i| Rng::new(i + 1).unit_vec3()).collect();
  dirs.extend([
    Vec3::new(1., 0., 0.),
    Vec3::new(0., 1., 0.),
    Vec3::new(0., 0., 1.),
    Vec3::new(0., 0., -1.),
    Vec3::new(0., -1., 0.),
  ]);
  for dir in dirs {
    let n = dir.into_normalized();
    let m = n.local_to_world();
    // maps the local z to the normal
    assert_near(m * Vec3::new(0., 0., 1.), dir);
    // orthonormal and right handed
    assert_near(m.transpose() * m, Mat3::identity());
    assert_near(m.det(), 1.);
  }
}

#[test]
fn swizzle() {
  let v = Vec4::new(1, 2, 3, 4);
  assert_eq!(v.x(), 1);
  assert_eq!(v.a(), 4);
  assert_eq!(v.xy(), Vec2::new(1, 2));
  assert_eq!(v.wzyx(), Vec4::new(4, 3, 2, 1));
  assert_eq!(v.xyz(), Vec3::new(1, 2, 3));
  assert_eq!(v.rgb(), Vec3::new(1, 2, 3));
  assert_eq!(v.xxww(), Vec4::new(1, 1, 4, 4));

  let v = Vec3::new(1, 2, 3);
  assert_eq!(v.zyx(), Vec3::new(3, 2, 1));
  assert_eq!(v.zx(), Vec2::new(3, 1));
  assert_eq!(v.xyzx(), Vec4::new(1, 2, 3, 1));
  assert_eq!(v.bgr(), Vec3::new(3, 2, 1));

  let v = Vec2::new(1, 2);
  assert_eq!(v.yx(), Vec2::new(2, 1));
  assert_eq!(v.xyx(), Vec3::new(1, 2, 1));
  assert_eq!(v.xyyx(), Vec4::new(1, 2, 2, 1));
  assert_eq!(v.gr(), Vec2::new(2, 1));
}

#[test]
fn conversion_and_index() {
  let mut v = Vec4::new(1, 2, 3, 4);
  assert_eq!(v[0], 1);
  assert_eq!(v[3], 4);
  assert_eq!(&v[1..3], &[2, 3]);
  assert_eq!(&v[..2], &[1, 2]);
  assert_eq!(&v[2..], &[3, 4]);
  assert_eq!(&v[..], &[1, 2, 3, 4]);
  v[1] = 5;
  v[2..].copy_from_slice(&[6, 7]);
  assert_eq!(v, Vec4::new(1, 5, 6, 7));

  let arr: [i32; 4] = v.into();
  assert_eq!(arr, [1, 5, 6, 7]);
  assert_eq!(Vec4::from([1, 5, 6, 7]), v);
  let r: &[i32; 4] = v.as_ref();
  assert_eq!(r, &[1, 5, 6, 7]);
  let as_mut: &mut [i32; 4] = v.as_mut();
  as_mut[0] = 9;
  assert_eq!(v.x, 9);

  let arr = [1., 2., 3.];
  let r: &Vec3<f32> = (&arr).into();
  assert_eq!(*r, Vec3::new(1., 2., 3.));
  let mut arr = [1., 2.];
  let r: &mut Vec2<f32> = (&mut arr).into();
  r.y = 5.;
  assert_eq!(arr, [1., 5.]);

  let t: (u8, u8, u8) = Vec3::new(1, 2, 3).into();
  assert_eq!(t, (1, 2, 3));
  assert_eq!(Vec2::from((1, 2)), Vec2::new(1, 2));
  assert_eq!(Vec3::new(1, 2, 3).expand_with(4), Vec4::new(1, 2, 3, 4));
  assert_eq!(Vec3::new(1, 2, 3).expand_with_one(), Vec4::new(1, 2, 3, 1));

  assert_eq!(vector!(1, 2), Vec2::new(1, 2));
  assert_eq!(vector!(1, 2, 3), vec3(1, 2, 3));
  assert_eq!(vector!(1, 2, 3, 4), vec4(1, 2, 3, 4));
  assert_eq!(vec2(1, 2), Vec2::new(1, 2));

  let v = Vec3::new(1.5_f32, 2., 3.);
  assert_eq!(v.into_f64(), Vec3::new(1.5_f64, 2., 3.));
  assert_eq!(v.into_f64().into_f32(), v);

  let v = Vec3::new(1_u32, 2, 3);
  let bytes = bytemuck::bytes_of(&v);
  assert_eq!(bytes.len(), 12);
  assert_eq!(std::mem::size_of::<Vec4<f32>>(), 16);

  assert_eq!(format!("{}", Vec2::new(1, 2)), "(1, 2)");
  assert_eq!(format!("{}", Vec3::new(1, 2, 3)), "(1, 2, 3)");
  assert_eq!(format!("{}", Vec4::new(1, 2, 3, 4)), "(1, 2, 3, 4)");
}

#[test]
fn f32_vec() {
  // make sure the f32 version is also consistent
  let a = Vec3::new(1.5_f32, -2., 0.25);
  let b = Vec3::new(-3_f32, 0.5, 4.);
  let (ga, gb) = (Vec3A::new(1.5, -2., 0.25), Vec3A::new(-3., 0.5, 4.));
  let c: [f32; 3] = a.cross(b).into();
  assert_eq!(c, ga.cross(gb).to_array());
  assert_eq!(a.dot(b), ga.dot(gb));
  let n: [f32; 3] = a.normalize().into();
  let gn = ga.normalize().to_array();
  for (x, y) in n.iter().zip(gn) {
    assert!((x - y).abs() < 1e-6);
  }
}
