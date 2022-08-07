use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct TorusMeshParameter {
  radius: f32,
  tube_radius: f32,
}

impl TorusMeshParameter {
  pub fn make_surface(self) -> impl ParametricSurface {
    let Self {
      radius,
      tube_radius,
    } = self;

    UnitCircle
      .transform_by(Mat3::scale(Vec2::splat(radius)))
      .embed_to_surface(ParametricPlane)
      .tube_by(UnitCircle.transform_by(Mat3::scale(Vec2::splat(tube_radius))))
  }
}

#[derive(Copy, Clone, Debug)]
pub struct CubeMeshParameter {
  /// span x axis
  pub width: f32,
  /// span y axis
  pub height: f32,
  /// span z axis
  pub depth: f32,
}

impl CubeMeshParameter {
  pub fn make_faces(self) -> [Transformed3D<ParametricPlane>; 6] {
    let Self {
      width,
      height,
      depth,
    } = self;

    let mat = |normal_move: (f32, f32, f32), rotate: Mat4<f32>| -> Mat4<f32> {
      let normal_move: Vec3<f32> = normal_move.into();
      let extend: Vec3<f32> = (width, height, depth).into();

      Mat4::translate(extend * normal_move) // push front or back
    * Mat4::scale(extend) // apply cube parameter by scaling
    * rotate // rotate to correct plane
    * Mat4::translate((-0.5, -0.5, 0.)) // move to center
    };

    [
      ParametricPlane.transform_by(mat((1., 0., 0.), Mat4::rotate_y(f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((-1., 0., 0.), Mat4::rotate_y(-f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((0., 1., 0.), Mat4::rotate_x(f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((0., -1., 0.), Mat4::rotate_x(-f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((0., 0., 1.), Mat4::one())),
      ParametricPlane.transform_by(mat((0., 0., -1.), Mat4::rotate_y(f32::PI()))),
    ]
  }
}

#[derive(Copy, Clone, Debug)]
pub struct SphereMeshParameter {
  pub radius: f32,
  /// in radius
  pub phi_start: f32,
  /// in radius
  pub phi_length: f32,
  /// in radius
  pub theta_start: f32,
  /// in radius
  pub theta_length: f32,
}

impl SphereMeshParameter {
  pub fn make_surface(self) -> impl ParametricSurface {
    let Self {
      radius,
      phi_start,
      phi_length,
      theta_start,
      theta_length,
    } = self;

    let to_normalized = 1. / (2. * f32::PI());
    let u_range = phi_start * to_normalized..(phi_start + phi_length) * to_normalized;
    let v_range = theta_start * to_normalized..(theta_start + theta_length) * to_normalized;

    UVSphere
      .transform_by(Mat4::scale(Vec3::splat(radius)))
      .map_range(u_range, v_range)
  }
}

#[derive(Copy, Clone, Debug)]
pub struct CylinderMeshParameter {
  pub radius_top: f32,
  pub radius_bottom: f32,
  pub height: f32,
  pub theta_start: f32,
  pub theta_length: f32,
}
