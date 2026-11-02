use glam::dcamera::rh::proj::{directx, opengl};
use rendiation_algebra::*;

use crate::util::*;

fn perspective() -> PerspectiveProjection<f64> {
  PerspectiveProjection {
    near: 0.5,
    far: 300.,
    fov: Deg::by(60.),
    aspect: 1.5,
  }
}

fn orthographic() -> OrthographicProjection<f64> {
  OrthographicProjection {
    left: -4.,
    right: 6.,
    top: 3.,
    bottom: -2.,
    near: 0.5,
    far: 100.,
  }
}

#[test]
fn perspective_matrix() {
  let p = perspective();
  let fov = p.fov.to_rad();

  let gl = p.compute_projection_mat(&OpenGLxNDC);
  assert_near(gl, opengl::perspective(fov, p.aspect, p.near, p.far));
  let wgpu = p.compute_projection_mat(&WebGPUxNDC);
  assert_near_eps(
    wgpu,
    directx::perspective(fov, p.aspect, p.near, p.far),
    1e-7,
  );

  // near plane maps to the ndc near, far plane maps to the ndc far
  assert_near((gl * Vec3::new(0., 0., -p.near)).z, -1.);
  assert_near_eps((gl * Vec3::new(0., 0., -p.far)).z, 1., 1e-7);
  assert_near((wgpu * Vec3::new(0., 0., -p.near)).z, 0.);
  assert_near_eps((wgpu * Vec3::new(0., 0., -p.far)).z, 1., 1e-7);

  assert!(gl.check_is_perspective_matrix_assume_common_projection());
  assert!(wgpu.check_is_perspective_matrix_assume_common_projection());
  let (n, f) = gl.get_near_far_assume_perspective();
  assert_near_eps(n, p.near, 1e-7);
  assert_near_eps(f, p.far, 1e-7);
  let (n, f) = gl.get_near_far_assume_is_common_projection();
  assert_near_eps(n, p.near, 1e-7);
  assert_near_eps(f, p.far, 1e-7);
}

#[test]
fn orthographic_matrix() {
  let p = orthographic();
  let (l, r, b, t, n, f) = (p.left, p.right, p.bottom, p.top, p.near, p.far);

  let gl = p.compute_projection_mat(&OpenGLxNDC);
  assert_near(gl, opengl::orthographic(l, r, b, t, n, f));
  let wgpu = p.compute_projection_mat(&WebGPUxNDC);
  assert_near(wgpu, directx::orthographic(l, r, b, t, n, f));

  // the view box corners map to the ndc corners
  assert_near(gl * Vec3::new(l, b, -n), Vec3::new(-1., -1., -1.));
  assert_near(gl * Vec3::new(r, t, -f), Vec3::new(1., 1., 1.));
  assert_near(wgpu * Vec3::new(r, t, -f), Vec3::new(1., 1., 1.));
  assert_near(wgpu * Vec3::new(l, b, -n), Vec3::new(-1., -1., 0.));

  assert!(!gl.check_is_perspective_matrix_assume_common_projection());
  let (near, far) = gl.get_near_far_assume_orthographic();
  assert_near_eps(near, n, 1e-7);
  assert_near_eps(far, f, 1e-7);
  let (near, far) = gl.get_near_far_assume_is_common_projection();
  assert_near_eps(near, n, 1e-7);
  assert_near_eps(far, f, 1e-7);
}

#[test]
fn ndc_mapper() {
  let to = NDCSpaceMapper::<f64>::transform_from_opengl_standard_ndc(&WebGPUxNDC);
  let back = NDCSpaceMapper::<f64>::transform_into_opengl_standard_ndc(&WebGPUxNDC);
  assert_near(to * back, Mat4::identity());
  assert_near(to * Vec3::new(0.3, 0.4, -1.), Vec3::new(0.3, 0.4, 0.));
  assert_near(to * Vec3::new(0.3, 0.4, 1.), Vec3::new(0.3, 0.4, 1.));
  assert_eq!(
    NDCSpaceMapper::<f64>::transform_from_opengl_standard_ndc(&OpenGLxNDC),
    Mat4::identity()
  );

  let boxed: Box<dyn NDCSpaceMapper<f32>> = Box::new(WebGPUxNDC);
  let cloned = boxed.clone();
  assert_eq!(
    cloned.transform_from_opengl_standard_ndc(),
    boxed.transform_from_opengl_standard_ndc()
  );
}

#[test]
fn pixels_per_unit() {
  let height = 720.;
  let p = perspective();
  for mapper in [&OpenGLxNDC as &dyn NDCSpaceMapper<f64>, &WebGPUxNDC] {
    let mat = p.compute_projection_mat(mapper);
    let inv = mat.inverse().unwrap();
    for distance in [1., 5., 20., 150.] {
      assert_near_eps(
        mat.pixels_per_unit(inv, distance, height),
        p.pixels_per_unit(distance, height),
        1e-6,
      );
    }
  }
  // the view height at distance d is 2 * d * tan(fov / 2)
  let d = 10.;
  let expect = height / (2. * d * (p.fov.to_rad() / 2.).tan());
  assert_near(p.pixels_per_unit(d, height), expect);

  let o = orthographic();
  let mat = o.compute_projection_mat(&OpenGLxNDC);
  let inv = mat.inverse().unwrap();
  assert_near_eps(
    mat.pixels_per_unit(inv, 10., height),
    o.pixels_per_unit(10., height),
    1e-7,
  );
  assert_near(o.pixels_per_unit(10., height), height / 5.);
}

#[test]
fn projection_utils() {
  let mut o = orthographic();
  assert_eq!(o.size(), Vec2::new(10., 5.));
  assert_eq!(o.center(), Vec2::new(1., 0.5));
  o.scale_from_center(Vec2::new(2., 0.5));
  assert_eq!(o.size(), Vec2::new(20., 2.5));
  assert_eq!(o.center(), Vec2::new(1., 0.5));

  let mut p = perspective();
  p.resize((1920., 1080.));
  assert_near(p.aspect, 1920. / 1080.);

  let d = PerspectiveProjection::<f32>::default();
  assert!(d.near > 0. && d.far > d.near);
  let d = OrthographicProjection::<f32>::default();
  assert_eq!(d.size(), Vec2::new(100., 100.));
}
