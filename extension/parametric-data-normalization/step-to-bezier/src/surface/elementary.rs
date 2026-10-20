//! Cylinder, cone, sphere and torus (ISO 10303-42 elementary surfaces). All are built with the
//! shared exact revolve construction around the frame `z` axis, the generatrix being the
//! `u = 0` iso-curve.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::*;

/// `CYLINDRICAL_SURFACE`: `o + r(cos u·x + sin u·y) + v·z`.
pub struct Cylinder {
  frame: Frame,
  radius: f64,
}

/// `CONICAL_SURFACE`: `o + (R + v·tan α)(cos u·x + sin u·y) + v·z`, apex at
/// `v = -R / tan α`.
pub struct Cone {
  frame: Frame,
  radius: f64,
  tan: f64,
}

/// `SPHERICAL_SURFACE`: `o + r·cos v(cos u·x + sin u·y) + r·sin v·z`, `v ∈ [-π/2, π/2]`.
pub struct Sphere {
  frame: Frame,
  radius: f64,
}

/// `TOROIDAL_SURFACE`: `o + (R + r·cos v)(cos u·x + sin u·y) + r·sin v·z`.
pub struct Torus {
  frame: Frame,
  major: f64,
  minor: f64,
}

impl Cylinder {
  pub fn new(frame: Frame, radius: f64) -> ConvertResult<Self> {
    check_positive(radius, "cylinder radius")?;
    Ok(Self { frame, radius })
  }
}

impl Cone {
  pub fn new(frame: Frame, radius: f64, semi_angle: f64) -> ConvertResult<Self> {
    let tan = semi_angle.tan();
    if !(radius.is_finite() && radius >= 0. && tan.is_finite()) {
      return Err(ConvertError::invalid(
        None,
        format!("invalid cone radius {radius} / semi angle {semi_angle}"),
      ));
    }
    if radius == 0. && tan == 0. {
      return Err(ConvertError::invalid(None, "degenerate cone"));
    }
    Ok(Self { frame, radius, tan })
  }

  fn apex(&self) -> Option<f64> {
    (self.tan != 0.).then(|| -self.radius / self.tan)
  }
}

impl Sphere {
  pub fn new(frame: Frame, radius: f64) -> ConvertResult<Self> {
    check_positive(radius, "sphere radius")?;
    Ok(Self { frame, radius })
  }
}

impl Torus {
  pub fn new(frame: Frame, major: f64, minor: f64) -> ConvertResult<Self> {
    check_positive(minor, "torus minor radius")?;
    if !(major.is_finite() && major >= 0.) {
      return Err(ConvertError::invalid(
        None,
        format!("invalid torus major radius {major}"),
      ));
    }
    Ok(Self {
      frame,
      major,
      minor,
    })
  }
}

fn check_positive(value: f64, what: &str) -> ConvertResult<()> {
  if !(value.is_finite() && value > 0.) {
    return Err(ConvertError::invalid(
      None,
      format!("invalid {what} {value}"),
    ));
  }
  Ok(())
}

fn frame_axis(frame: &Frame) -> RevolveAxis {
  RevolveAxis {
    origin: frame.origin,
    dir: frame.z,
  }
}

/// Radial direction at angle `u` and its derivative.
fn radial(frame: &Frame, u: f64) -> (Vec3<f64>, Vec3<f64>) {
  let (s, c) = u.sin_cos();
  (frame.x * c + frame.y * s, frame.y * c - frame.x * s)
}

/// Frame local coordinates of `p`: distance to the axis, angle around it, height.
fn cylindrical(frame: &Frame, p: Vec3<f64>) -> (f64, f64, f64) {
  let d = p - frame.origin;
  let (x, y) = (d.dot(frame.x), d.dot(frame.y));
  (x.hypot(y), y.atan2(x), d.dot(frame.z))
}

/// Angle at a point on the axis (any angle is valid): the guess when given.
fn axis_angle(angle: f64, on_axis: bool, guess: Option<f64>) -> f64 {
  if on_axis { guess.unwrap_or(0.) } else { angle }
}

fn angle_near(angle: f64, guess: Option<f64>) -> f64 {
  nearest_representative(angle, Some(TAU), guess)
}

fn finish(s: &dyn SurfaceModel, p: Vec3<f64>, native: Vec2<f64>) -> Option<SurfaceProjection> {
  finite_projection(native, (s.eval(native) - p).length())
}

fn line_generatrix(
  frame: &Frame,
  radius: impl Fn(f64) -> f64,
) -> impl Fn(f64, f64) -> RationalBezierCurve3 {
  let frame = *frame;
  move |v0, v1| {
    let at = |v: f64| frame.origin + frame.x * radius(v) + frame.z * v;
    RationalBezierCurve3::line(at(v0), at(v1))
  }
}

impl SurfaceModel for Cylinder {
  fn kind_name(&self) -> &'static str {
    "Cylinder"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    let (e, _) = radial(&self.frame, native.x);
    self.frame.origin + e * self.radius + self.frame.z * native.y
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (e, de) = radial(&self.frame, native.x);
    (
      self.frame.origin + e * self.radius + self.frame.z * native.y,
      de * self.radius,
      self.frame.z,
    )
  }

  fn period(&self) -> [Option<f64>; 2] {
    [Some(TAU), None]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, None]
  }

  fn poles(&self) -> Vec<Pole> {
    Vec::new()
  }

  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let (rho, angle, h) = cylindrical(&self.frame, p);
    let on_axis = rho <= 1e-14 * (self.radius + h.abs());
    let u = angle_near(
      axis_angle(angle, on_axis, guess.map(|g| g.x)),
      guess.map(|g| g.x),
    );
    finish(self, p, Vec2::new(u, h))
  }

  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let u_map = angular_map(rect.u, ctx)?;
    let v_map = affine_map(rect.v)?;
    let r = self.radius;
    let generatrix = analytic_generatrix(&v_map, line_generatrix(&self.frame, |_| r));
    Ok(revolve_param_space(
      &frame_axis(&self.frame),
      u_map,
      v_map,
      &generatrix,
    ))
  }
}

impl SurfaceModel for Cone {
  fn kind_name(&self) -> &'static str {
    "Cone"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    let (e, _) = radial(&self.frame, native.x);
    self.frame.origin + e * (self.radius + native.y * self.tan) + self.frame.z * native.y
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (e, de) = radial(&self.frame, native.x);
    let rho = self.radius + native.y * self.tan;
    (
      self.frame.origin + e * rho + self.frame.z * native.y,
      de * rho,
      e * self.tan + self.frame.z,
    )
  }

  fn period(&self) -> [Option<f64>; 2] {
    [Some(TAU), None]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, None]
  }

  fn poles(&self) -> Vec<Pole> {
    self
      .apex()
      .map(|v| Pole {
        axis: Axis::V,
        value: v,
        position: self.frame.origin + self.frame.z * v,
      })
      .into_iter()
      .collect()
  }

  /// Both nappes are considered: in the meridian half plane through `p` the surface is the
  /// line `ρ = R + v·tan α` where that radius is positive, and its mirror `ρ = -(R + v·tan α)`
  /// (angle `u + π`) where it is negative.
  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let (rho, angle, h) = cylindrical(&self.frame, p);
    let (r, t) = (self.radius, self.tan);
    let n2 = t * t + 1.;
    let mut candidates = vec![(((rho - r) * t + h) / n2, false)];
    if let Some(apex) = self.apex() {
      candidates.push(((-(rho + r) * t + h) / n2, true));
      for (v, flip) in &mut candidates {
        let s = r + *v * t;
        if (!*flip && s < 0.) || (*flip && s > 0.) {
          *v = apex;
        }
      }
    }
    let dist2 = |v: f64| (rho - (r + v * t).abs()).powi(2) + (h - v).powi(2);
    let (v, flip) = candidates
      .into_iter()
      .min_by(|a, b| dist2(a.0).total_cmp(&dist2(b.0)))?;
    let (v, at_apex) = match self.apex() {
      Some(apex) if (r + v * t).abs() <= 1e-14 * (r + (v * t).abs()) => (apex, true),
      apex => (v, apex == Some(v)),
    };
    let on_axis = at_apex || rho <= 1e-14 * (r + h.abs() + rho);
    let angle = if flip { angle + PI } else { angle };
    let u = angle_near(
      axis_angle(angle, on_axis, guess.map(|g| g.x)),
      guess.map(|g| g.x),
    );
    finish(self, p, Vec2::new(u, v))
  }

  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let u_map = angular_map(rect.u, ctx)?;
    let v_map = affine_map(rect.v)?;
    let (r, t) = (self.radius, self.tan);
    let generatrix = analytic_generatrix(&v_map, line_generatrix(&self.frame, |v| r + v * t));
    Ok(revolve_param_space(
      &frame_axis(&self.frame),
      u_map,
      v_map,
      &generatrix,
    ))
  }
}

impl SurfaceModel for Sphere {
  fn kind_name(&self) -> &'static str {
    "Sphere"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    self.eval_with_partials(native).0
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (e, de) = radial(&self.frame, native.x);
    let (s, c) = native.y.sin_cos();
    let r = self.radius;
    (
      self.frame.origin + e * (r * c) + self.frame.z * (r * s),
      de * (r * c),
      self.frame.z * (r * c) - e * (r * s),
    )
  }

  fn period(&self) -> [Option<f64>; 2] {
    [Some(TAU), None]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, Some((-FRAC_PI_2, FRAC_PI_2))]
  }

  fn poles(&self) -> Vec<Pole> {
    [-1., 1.]
      .into_iter()
      .map(|s| Pole {
        axis: Axis::V,
        value: FRAC_PI_2 * s,
        position: self.frame.origin + self.frame.z * (self.radius * s),
      })
      .collect()
  }

  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let (rho, angle, h) = cylindrical(&self.frame, p);
    let len = rho.hypot(h);
    let guess_u = guess.map(|g| g.x);
    if len <= 1e-14 * self.radius {
      let native = guess.unwrap_or(Vec2::new(0., 0.));
      return finish(
        self,
        p,
        Vec2::new(native.x, native.y.clamp(-FRAC_PI_2, FRAC_PI_2)),
      );
    }
    let on_axis = rho <= 1e-14 * len;
    let v = if on_axis {
      FRAC_PI_2.copysign(h)
    } else {
      h.atan2(rho)
    };
    let u = angle_near(axis_angle(angle, on_axis, guess_u), guess_u);
    finish(self, p, Vec2::new(u, v))
  }

  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let eps = 1e-9;
    if rect.v.0 < -FRAC_PI_2 - eps || rect.v.1 > FRAC_PI_2 + eps {
      return Err(ConvertError::numerical(
        None,
        format!("sphere latitude range {:?} outside [-π/2, π/2]", rect.v),
      ));
    }
    let u_map = angular_map(rect.u, ctx)?;
    let v_map = bounded_angular_map((rect.v.0.max(-FRAC_PI_2), rect.v.1.min(FRAC_PI_2)))?;
    let f = self.frame;
    let r = self.radius;
    let generatrix = analytic_generatrix(&v_map, |b0, b1| {
      elliptic_arc_3d(f.origin, f.x, f.z, r, r, b0, b1)
    });
    Ok(revolve_param_space(
      &frame_axis(&f),
      u_map,
      v_map,
      &generatrix,
    ))
  }
}

impl SurfaceModel for Torus {
  fn kind_name(&self) -> &'static str {
    "Torus"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    self.eval_with_partials(native).0
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (e, de) = radial(&self.frame, native.x);
    let (s, c) = native.y.sin_cos();
    let rho = self.major + self.minor * c;
    (
      self.frame.origin + e * rho + self.frame.z * (self.minor * s),
      de * rho,
      self.frame.z * (self.minor * c) - e * (self.minor * s),
    )
  }

  fn period(&self) -> [Option<f64>; 2] {
    [Some(TAU), Some(TAU)]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, None]
  }

  /// A horn torus (`R == 0`, degenerate) is not reported, the minor circle never shrinks.
  fn poles(&self) -> Vec<Pole> {
    Vec::new()
  }

  /// The minor circle in the meridian half plane through `p` (center at radius `R`) and its
  /// mirror in the opposite half plane (angle `u + π`) are both candidates, which matters for
  /// spindle tori only.
  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let (rho, angle, h) = cylindrical(&self.frame, p);
    let (big, small) = (self.major, self.minor);
    let near = (rho - big).hypot(h);
    let far = (rho + big).hypot(h);
    let flip = (far - small).abs() < (near - small).abs();
    let (angle, x, center_dist) = if flip {
      (angle + PI, -rho - big, far)
    } else {
      (angle, rho - big, near)
    };
    let scale = big + small + rho.abs() + h.abs();
    let on_axis = rho <= 1e-14 * scale;
    let guess_u = guess.map(|g| g.x);
    let guess_v = guess.map(|g| g.y);
    let u = angle_near(axis_angle(angle, on_axis, guess_u), guess_u);
    let v = if center_dist <= 1e-14 * scale {
      guess_v.unwrap_or(0.)
    } else {
      h.atan2(x)
    };
    let v = angle_near(v, guess_v);
    finish(self, p, Vec2::new(u, v))
  }

  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let u_map = angular_map(rect.u, ctx)?;
    let v_map = angular_map(rect.v, ctx)?;
    let f = self.frame;
    let (big, small) = (self.major, self.minor);
    let generatrix = analytic_generatrix(&v_map, |b0, b1| {
      elliptic_arc_3d(f.origin + f.x * big, f.x, f.z, small, small, b0, b1)
    });
    Ok(revolve_param_space(
      &frame_axis(&f),
      u_map,
      v_map,
      &generatrix,
    ))
  }
}
