use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct TorusMeshParameter {
  pub radius: f32,
  pub tube_radius: f32,
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

impl Default for CubeMeshParameter {
  fn default() -> Self {
    Self {
      width: 1.,
      height: 1.,
      depth: 1.,
    }
  }
}

impl CubeMeshParameter {
  pub fn make_faces(&self) -> [Transformed3D<ParametricPlane>; 6] {
    let Self {
      width,
      height,
      depth,
    } = *self;

    let mat = |normal_move: (f32, f32, f32), rotate: Mat4<f32>| -> Mat4<f32> {
      let normal_move: Vec3<f32> = normal_move.into();
      let extend: Vec3<f32> = (width, height, depth).into();

      Mat4::translate(extend * normal_move / 2.) // push front or back
    * Mat4::scale(extend) // apply cube parameter by scaling
    * rotate // rotate to correct plane
    * Mat4::translate((-0.5, -0.5, 0.)) // move to center
    };

    [
      ParametricPlane.transform_by(mat((1., 0., 0.), Mat4::rotate_y(f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((-1., 0., 0.), Mat4::rotate_y(-f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((0., 1., 0.), Mat4::rotate_x(-f32::PI() / 2.))),
      ParametricPlane.transform_by(mat((0., -1., 0.), Mat4::rotate_x(f32::PI() / 2.))),
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

impl Default for SphereMeshParameter {
  fn default() -> Self {
    Self {
      radius: 1.0,
      phi_start: 0.,
      phi_length: f32::PI() * 2.,
      theta_start: 0.,
      theta_length: f32::PI(),
    }
  }
}

impl SphereMeshParameter {
  pub fn make_surface(&self) -> impl ParametricSurface {
    let Self {
      radius,
      phi_start,
      phi_length,
      theta_start,
      theta_length,
    } = *self;

    let to_normalized_u = 1. / (2. * f32::PI());
    let to_normalized_v = 1. / f32::PI();
    let u_range = phi_start * to_normalized_u..(phi_start + phi_length) * to_normalized_u;
    let v_range = theta_start * to_normalized_v..(theta_start + theta_length) * to_normalized_v;

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

impl Default for CylinderMeshParameter {
  fn default() -> Self {
    Self {
      radius_top: 1.,
      radius_bottom: 1.,
      height: 1.,
      theta_start: 0.,
      theta_length: 2. * f32::PI(),
    }
  }
}

type CylinderSurface = impl ParametricSurface;
impl CylinderMeshParameter {
  pub fn body_surface(&self) -> CylinderSurface {
    let Self {
      radius_top,
      radius_bottom,
      height,
      theta_start,
      theta_length,
    } = *self;

    let to_normalized = 1. / (2. * f32::PI());
    let range = theta_start * to_normalized..(theta_start + theta_length) * to_normalized;

    LineSegment2D {
      start: Vec2::new(radius_bottom, 0.),
      end: Vec2::new(radius_top, height),
    }
    .rotate_sweep()
    .map_range(0.0..1.0, range)
  }

  pub fn cap_surface(&self, top: bool) -> Option<CylinderSurface> {
    let Self {
      radius_top,
      radius_bottom,
      height,
      theta_start,
      theta_length,
    } = *self;

    let (radius, height) = if top {
      (radius_top, height)
    } else {
      (radius_bottom, 0.)
    };

    if radius == 0. {
      None
    } else {
      let to_normalized = 1. / (2. * f32::PI());
      let range = theta_start * to_normalized..(theta_start + theta_length) * to_normalized;

      LineSegment2D {
        start: Vec2::new(0., height),
        end: Vec2::new(radius, height),
      }
      .rotate_sweep()
      .map_range(0.0..1.0, range)
      .into()
    }
  }
}
