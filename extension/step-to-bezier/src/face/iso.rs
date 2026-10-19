//! Analytic proofs that a 3D edge curve is an iso-line of an analytic surface, from the
//! `CurveHint` of the edge. Native axis 0 is `u`, 1 is `v`.

use super::*;

/// The native axis whose coordinate stays constant along the curve, when the curve geometry
/// guarantees it: lines parallel to a cylinder axis or an extrusion direction, lines through
/// a cone apex (generators), circles coaxial with a surface of revolution (parallels), and
/// meridian circles of spheres and tori.
pub(super) fn hint_iso(geom: &SurfaceGeom, hint: &CurveHint, ctx: &ConvertCtx) -> Option<usize> {
  let eps = IsoEps {
    angle: ctx.angular_tol.max(1e-12),
    dist: super::native::EXACT_FACTOR * ctx.fit_tol,
  };
  match (geom, hint) {
    (SurfaceGeom::Cylinder { frame, .. }, CurveHint::Line { dir, .. }) => {
      eps.parallel(*dir, frame.z).then_some(0)
    }
    (SurfaceGeom::Extrusion { vector, .. }, CurveHint::Line { dir, .. }) => {
      eps.parallel(*dir, *vector).then_some(0)
    }
    (
      SurfaceGeom::Cone {
        frame,
        radius,
        semi_angle,
      },
      CurveHint::Line { origin, dir },
    ) => {
      let tan = semi_angle.tan();
      if !(tan.is_finite() && tan != 0.) {
        return None;
      }
      let apex = frame.origin + frame.z * (-radius / tan);
      let dir = dir.normalize();
      ((apex - *origin).cross(dir).length() <= eps.dist).then_some(0)
    }
    (
      SurfaceGeom::Cylinder { frame, .. }
      | SurfaceGeom::Cone { frame, .. }
      | SurfaceGeom::Sphere { frame, .. }
      | SurfaceGeom::Torus { frame, .. },
      CurveHint::Conic {
        frame: circle,
        rx,
        ry,
      },
    ) if eps.circle(*rx, *ry) => {
      if eps.coaxial(circle, frame.origin, frame.z) {
        return Some(1);
      }
      meridian(geom, frame, circle, *rx, &eps).then_some(0)
    }
    (
      SurfaceGeom::Revolution {
        axis_origin,
        axis_dir,
        ..
      },
      CurveHint::Conic {
        frame: circle,
        rx,
        ry,
      },
    ) if eps.circle(*rx, *ry) => eps.coaxial(circle, *axis_origin, *axis_dir).then_some(1),
    _ => None,
  }
}

struct IsoEps {
  angle: f64,
  dist: f64,
}

impl IsoEps {
  fn parallel(&self, a: Vec3<f64>, b: Vec3<f64>) -> bool {
    let (a, b) = (a.normalize(), b.normalize());
    a.cross(b).length() <= self.angle
  }

  fn perpendicular(&self, a: Vec3<f64>, b: Vec3<f64>) -> bool {
    a.normalize().dot(b.normalize()).abs() <= self.angle
  }

  fn circle(&self, rx: f64, ry: f64) -> bool {
    (rx - ry).abs() <= self.dist
  }

  /// Circle plane perpendicular to the axis and center on the axis.
  fn coaxial(&self, circle: &Frame, origin: Vec3<f64>, axis: Vec3<f64>) -> bool {
    let axis = axis.normalize();
    self.parallel(circle.z, axis) && (circle.origin - origin).cross(axis).length() <= self.dist
  }
}

/// Meridian circles: on a sphere a great circle in a plane containing the axis, on a torus a
/// minor circle centered on the major circle in a plane containing the axis.
fn meridian(geom: &SurfaceGeom, frame: &Frame, circle: &Frame, radius: f64, eps: &IsoEps) -> bool {
  match geom {
    SurfaceGeom::Sphere { radius: r, .. } => {
      (circle.origin - frame.origin).length() <= eps.dist
        && (radius - r).abs() <= eps.dist
        && eps.perpendicular(circle.z, frame.z)
    }
    SurfaceGeom::Torus {
      major_radius,
      minor_radius,
      ..
    } => {
      let d = circle.origin - frame.origin;
      let height = d.dot(frame.z);
      let radial = d - frame.z * height;
      height.abs() <= eps.dist
        && (radial.length() - major_radius).abs() <= eps.dist
        && (radius - minor_radius).abs() <= eps.dist
        && eps.perpendicular(circle.z, frame.z)
        && eps.perpendicular(circle.z, radial)
    }
    _ => false,
  }
}
