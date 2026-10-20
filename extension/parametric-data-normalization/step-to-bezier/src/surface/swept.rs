//! Swept surfaces: `SURFACE_OF_LINEAR_EXTRUSION` and `SURFACE_OF_REVOLUTION`.

use std::f64::consts::TAU;

use super::*;

/// `C(u) + v·V`: `u` is the swept curve parameter, `v` a multiple of the extrusion vector.
pub struct Extrusion {
  curve: SweptCurve,
  vector: Vec3<f64>,
  tol: f64,
}

/// ISO 10303-42: `S(u, v) = C + (λ(v) - C)·cos u + ((λ(v) - C)·V)·V·(1 - cos u)
/// + V × (λ(v) - C)·sin u`, the generatrix `λ` rotated by `u` right handed about the axis
/// `(C, V)`. `u = 0` is the generatrix itself.
pub struct Revolution {
  curve: SweptCurve,
  axis: RevolveAxis,
  poles: Vec<Pole>,
  tol: f64,
}

/// The swept curve of a swept surface, resolved once so the surface does not borrow the
/// model.
pub enum SweptCurve {
  /// Unbounded line `origin + t·vector`: the axis map is one affine segment per face range.
  Line {
    origin: Vec3<f64>,
    vector: Vec3<f64>,
  },
  /// Full circle or ellipse `o + rx·cos t·x + ry·sin t·y`: angular maps per face range like
  /// the analytic surfaces.
  Conic { frame: Frame, rx: f64, ry: f64 },
  /// Any other curve: the fixed param space of the whole curve from
  /// `curve::build_curve_param_space` (periodic when the curve is closed), cells intersecting
  /// the face range are produced.
  Fixed {
    map: PiecewiseParamMap,
    segments: Vec<RationalBezierCurve3>,
  },
}

impl SweptCurve {
  pub fn new(model: &BrepModel, curve: CurveId, ctx: &ConvertCtx) -> ConvertResult<Self> {
    let geom = model
      .curves
      .get(curve.index())
      .ok_or_else(|| ConvertError::invalid(None, format!("dangling curve id {curve:?}")))?;
    match geom {
      CurveGeom::Line { origin, vector } => Self::line(*origin, *vector),
      CurveGeom::Circle { frame, radius } => Self::conic(*frame, *radius, *radius),
      CurveGeom::Ellipse {
        frame,
        semi_axis1,
        semi_axis2,
      } => Self::conic(*frame, *semi_axis1, *semi_axis2),
      _ => {
        let space = crate::curve::build_curve_param_space(model, curve, None, ctx)?;
        Self::fixed(space.map, space.segments, ctx.tol)
      }
    }
  }

  pub fn line(origin: Vec3<f64>, vector: Vec3<f64>) -> ConvertResult<Self> {
    let l = vector.length();
    if !(l > 0. && l.is_finite()) {
      return Err(ConvertError::invalid(
        None,
        "swept line with zero direction",
      ));
    }
    Ok(Self::Line { origin, vector })
  }

  pub fn conic(frame: Frame, rx: f64, ry: f64) -> ConvertResult<Self> {
    if !(rx > 0. && ry > 0. && rx.is_finite() && ry.is_finite()) {
      return Err(ConvertError::invalid(
        None,
        format!("invalid swept conic radii {rx} {ry}"),
      ));
    }
    Ok(Self::Conic { frame, rx, ry })
  }

  /// A fixed param space, made periodic when its ends coincide within `tol`.
  pub fn fixed(
    mut map: PiecewiseParamMap,
    segments: Vec<RationalBezierCurve3>,
    tol: f64,
  ) -> ConvertResult<Self> {
    let n = map.segment_count();
    if n == 0 || segments.len() != n {
      return Err(ConvertError::invalid(
        None,
        "swept curve param space does not match its segments",
      ));
    }
    if map.breaks.iter().any(|b| !b.is_finite()) || map.breaks.windows(2).any(|w| w[0] >= w[1]) {
      return Err(ConvertError::invalid(
        None,
        "swept curve param space breaks are not increasing",
      ));
    }
    if map.period.is_none() && (segments[0].start() - segments[n - 1].end()).length() <= tol {
      map.period = Some(map.breaks[n] - map.breaks[0]);
    }
    Ok(Self::Fixed { map, segments })
  }

  /// Point and derivative at native `t`.
  pub fn eval(&self, t: f64) -> (Vec3<f64>, Vec3<f64>) {
    match self {
      Self::Line { origin, vector } => (*origin + *vector * t, *vector),
      Self::Conic { frame, rx, ry } => {
        let (s, c) = t.sin_cos();
        (
          frame.origin + frame.x * (rx * c) + frame.y * (ry * s),
          frame.y * (ry * c) - frame.x * (rx * s),
        )
      }
      Self::Fixed { map, segments } => {
        let (cell, local, dt) = map_local(map, t);
        let (p, d) = segments[segment_of_cell(map, cell)].eval_with_derivative(local);
        (p, d * dt)
      }
    }
  }

  pub fn period(&self) -> Option<f64> {
    match self {
      Self::Line { .. } => None,
      Self::Conic { .. } => Some(TAU),
      Self::Fixed { map, .. } => map.period,
    }
  }

  /// `(start, period)` of a periodic curve.
  fn periodic(&self) -> Option<(f64, f64)> {
    match self {
      Self::Line { .. } => None,
      Self::Conic { .. } => Some((0., TAU)),
      Self::Fixed { map, .. } => map.period.map(|p| (map.breaks[0], p)),
    }
  }

  /// Parameter bounds of a bounded, non periodic curve.
  pub fn natural_domain(&self) -> Option<(f64, f64)> {
    match self {
      Self::Fixed { map, .. } if map.period.is_none() => {
        Some((map.breaks[0], map.breaks[map.segment_count()]))
      }
      _ => None,
    }
  }

  /// P space map of the swept axis for the face range and the curve segment of every needed
  /// cell. For `Fixed` the range must satisfy the `fixed_map_cells` precondition.
  pub fn param_axis(
    &self,
    range: (f64, f64),
    ctx: &ConvertCtx,
  ) -> ConvertResult<(PiecewiseParamMap, Vec<(usize, RationalBezierCurve3)>)> {
    match self {
      Self::Line { .. } => {
        let map = affine_map(range)?;
        let segment = RationalBezierCurve3::line(self.eval(range.0).0, self.eval(range.1).0);
        Ok((map, vec![(0, segment)]))
      }
      Self::Conic { frame, rx, ry } => {
        let map = angular_map(range, ctx)?;
        let segments = (0..map.segment_count())
          .map(|i| {
            let arc = elliptic_arc_3d(
              frame.origin,
              frame.x,
              frame.y,
              *rx,
              *ry,
              map.breaks[i],
              map.breaks[i + 1],
            );
            (i, arc)
          })
          .collect();
        Ok((map, segments))
      }
      Self::Fixed { map, segments } => {
        let cells = fixed_map_cells(map, range)?
          .into_iter()
          .map(|c| (c, segments[segment_of_cell(map, c as i64)].clone()))
          .collect();
        Ok((map.clone(), cells))
      }
    }
  }

  /// The whole curve (one period) as a map and one segment per cell, for global searches.
  /// `None` for unbounded lines.
  fn full_axis(&self) -> Option<(PiecewiseParamMap, Vec<RationalBezierCurve3>)> {
    match self {
      Self::Line { .. } => None,
      Self::Conic { frame, rx, ry } => {
        let map = PiecewiseParamMap::new(
          (0..=4).map(|i| TAU * i as f64 / 4.).collect(),
          vec![SegmentMap::CircularArc; 4],
          Some(TAU),
        );
        let segments = (0..4)
          .map(|i| {
            let (a0, a1) = (map.breaks[i], map.breaks[i + 1]);
            elliptic_arc_3d(frame.origin, frame.x, frame.y, *rx, *ry, a0, a1)
          })
          .collect();
        Some((map, segments))
      }
      Self::Fixed { map, segments } => Some((map.clone(), segments.clone())),
    }
  }
}

impl Extrusion {
  pub fn new(curve: SweptCurve, vector: Vec3<f64>, ctx: &ConvertCtx) -> ConvertResult<Self> {
    let l = vector.length();
    if !(l > 0. && l.is_finite()) {
      return Err(ConvertError::invalid(None, "zero extrusion vector"));
    }
    if let SweptCurve::Line { vector: a, .. } = &curve
      && a.cross(vector).length() <= 1e-12 * a.length() * l
    {
      return Err(ConvertError::invalid(
        None,
        "extrusion of a line along its own direction",
      ));
    }
    Ok(Self {
      curve,
      vector,
      tol: ctx.tol,
    })
  }

  fn seed(&self, p: Vec3<f64>) -> Vec<Vec2<f64>> {
    let vv = self.vector.dot(self.vector);
    if let SweptCurve::Line { origin, vector: a } = &self.curve {
      let d = p - *origin;
      let (m00, m01, m11) = (a.dot(*a), a.dot(self.vector), vv);
      let (r0, r1) = (a.dot(d), self.vector.dot(d));
      let det = m00 * m11 - m01 * m01;
      return vec![Vec2::new(
        (m11 * r0 - m01 * r1) / det,
        (m00 * r1 - m01 * r0) / det,
      )];
    }
    let Some((map, segments)) = self.curve.full_axis() else {
      return Vec::new();
    };
    // Orthogonal projection along the extrusion vector is linear, so the projected segments
    // are exact rational curves.
    let dir = self.vector * (1. / vv.sqrt());
    let flatten = |q: Vec3<f64>| q - dir * q.dot(dir);
    let projected: Vec<_> = segments
      .iter()
      .map(|s| {
        RationalBezierCurve3::new(
          s.control_points
            .iter()
            .map(|c| Vec4::from_euclid(flatten(c.to_euclid()), c.w))
            .collect(),
        )
      })
      .collect();
    let Some((i, t, _)) = closest_point_on_chain(&projected, flatten(p)) else {
      return Vec::new();
    };
    let u = map.to_native(i as f64 + t);
    let v = (p - self.curve.eval(u).0).dot(self.vector) / vv;
    vec![Vec2::new(u, v)]
  }
}

impl SurfaceModel for Extrusion {
  fn kind_name(&self) -> &'static str {
    "Extrusion"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    self.curve.eval(native.x).0 + self.vector * native.y
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (c, dc) = self.curve.eval(native.x);
    (c + self.vector * native.y, dc, self.vector)
  }

  fn period(&self) -> [Option<f64>; 2] {
    [self.curve.period(), None]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [self.curve.natural_domain(), None]
  }

  fn period_start(&self) -> [Option<f64>; 2] {
    [self.curve.periodic().map(|p| p.0), None]
  }

  fn poles(&self) -> Vec<Pole> {
    Vec::new()
  }

  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let axes = SearchAxes {
      bounds: [self.curve.natural_domain(), None],
      periodic: [self.curve.periodic(), None],
    };
    project_numerically(self, axes, self.tol, p, guess, || self.seed(p))
  }

  /// Precondition for curves with a fixed map (see `SweptCurve::Fixed`): `rect.u` starts in
  /// the base period of a closed curve, or lies inside the domain of an open one.
  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let (u_map, cells) = self.curve.param_axis(rect.u, ctx)?;
    let v_map = affine_map(rect.v)?;
    let (v0, v1) = rect.v;
    let patches = cells
      .into_iter()
      .map(|(i, segment)| {
        let mut cp = Vec::with_capacity(2 * segment.control_points.len());
        for offset in [self.vector * v0, self.vector * v1] {
          cp.extend(
            segment
              .control_points
              .iter()
              .map(|c| Vec4::from_euclid(c.to_euclid() + offset, c.w)),
          );
        }
        CellPatch {
          cell: (i, 0),
          surface: RationalBezierSurface::new(segment.degree(), 1, cp),
        }
      })
      .collect();
    Ok(SurfaceParamSpace {
      u_map,
      v_map,
      patches,
    })
  }
}

impl Revolution {
  pub fn new(
    curve: SweptCurve,
    axis_origin: Vec3<f64>,
    axis_dir: Vec3<f64>,
    ctx: &ConvertCtx,
  ) -> ConvertResult<Self> {
    let l = axis_dir.length();
    if !(l > 0. && l.is_finite()) {
      return Err(ConvertError::invalid(None, "zero revolution axis"));
    }
    let axis = RevolveAxis {
      origin: axis_origin,
      dir: axis_dir * (1. / l),
    };
    let poles = revolution_poles(&curve, &axis, ctx.tol);
    Ok(Self {
      curve,
      axis,
      poles,
      tol: ctx.tol,
    })
  }

  /// Rotation angle carrying the radial part of `q` onto the radial part of `p`.
  fn angle_between(&self, q: Vec3<f64>, p: Vec3<f64>) -> f64 {
    let (rq, rp) = (self.axis.split(q).1, self.axis.split(p).1);
    let a = self.axis.dir.dot(rq.cross(rp)).atan2(rq.dot(rp));
    if a.is_finite() { a } else { 0. }
  }

  fn seeds(&self, p: Vec3<f64>) -> Vec<Vec2<f64>> {
    let height = |q: Vec3<f64>| (q - self.axis.origin).dot(self.axis.dir);
    let (rho_p, z_p) = (self.axis.split(p).1.length(), height(p));
    if let SweptCurve::Line { origin, vector } = &self.curve {
      let mut vs = vec![(p - *origin).dot(*vector) / vector.dot(*vector)];
      let rate = vector.dot(self.axis.dir);
      if rate.abs() > 1e-12 * vector.length() {
        vs.push((z_p - height(*origin)) / rate);
      }
      return vs
        .into_iter()
        .map(|v| Vec2::new(self.angle_between(self.curve.eval(v).0, p), v))
        .collect();
    }
    let Some((map, segments)) = self.curve.full_axis() else {
      return Vec::new();
    };
    let mut best: Vec<_> = segments
      .iter()
      .enumerate()
      .map(|(k, segment)| {
        let m = 8 * (segment.degree() + 1);
        (0..=m)
          .map(|i| {
            let t = i as f64 / m as f64;
            let q = segment.eval(t);
            let d = (rho_p - self.axis.split(q).1.length()).powi(2) + (z_p - height(q)).powi(2);
            (d, k as f64 + t, q)
          })
          .min_by(|a, b| a.0.total_cmp(&b.0))
          .unwrap_or((f64::INFINITY, k as f64, segment.start()))
      })
      .collect();
    best.sort_by(|a, b| a.0.total_cmp(&b.0));
    best
      .into_iter()
      .take(4)
      .map(|(_, pv, q)| Vec2::new(self.angle_between(q, p), map.to_native(pv)))
      .collect()
  }
}

/// Generatrix points on the axis: open curve ends, or the point where a line meets the axis.
/// Points of a conic on the axis are not detected.
fn revolution_poles(curve: &SweptCurve, axis: &RevolveAxis, tol: f64) -> Vec<Pole> {
  let pole = |v: f64, q: Vec3<f64>| {
    let (foot, radial) = axis.split(q);
    (radial.length() <= tol).then_some(Pole {
      axis: Axis::V,
      value: v,
      position: foot,
    })
  };
  match curve {
    SweptCurve::Line { origin, vector } => {
      let r0 = axis.split(*origin).1;
      let ra = *vector - axis.dir * vector.dot(axis.dir);
      let rr = ra.dot(ra);
      if rr.is_nan() || rr <= 0. {
        return Vec::new();
      }
      let t = -r0.dot(ra) / rr;
      pole(t, *origin + *vector * t).into_iter().collect()
    }
    SweptCurve::Conic { .. } => Vec::new(),
    SweptCurve::Fixed { .. } => match curve.natural_domain() {
      Some((lo, hi)) => [lo, hi]
        .into_iter()
        .filter_map(|v| pole(v, curve.eval(v).0))
        .collect(),
      None => Vec::new(),
    },
  }
}

impl SurfaceModel for Revolution {
  fn kind_name(&self) -> &'static str {
    "Revolution"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    let (q, _) = self.curve.eval(native.y);
    self.axis.rotate_scaled(q, native.x, 1.)
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let (q, dq) = self.curve.eval(native.y);
    let (s, su) = self.axis.rotate_with_derivative(q, native.x);
    (s, su, self.axis.rotate_vector(dq, native.x))
  }

  fn period(&self) -> [Option<f64>; 2] {
    [Some(TAU), self.curve.period()]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [None, self.curve.natural_domain()]
  }

  fn period_start(&self) -> [Option<f64>; 2] {
    [Some(0.), self.curve.periodic().map(|p| p.0)]
  }

  fn poles(&self) -> Vec<Pole> {
    self.poles.clone()
  }

  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    let axes = SearchAxes {
      bounds: [None, self.curve.natural_domain()],
      periodic: [Some((0., TAU)), self.curve.periodic()],
    };
    project_numerically(self, axes, self.tol, p, guess, || self.seeds(p))
  }

  /// Precondition for generatrices with a fixed map (see `SweptCurve::Fixed`): `rect.v` starts
  /// in the base period of a closed curve, or lies inside the domain of an open one.
  fn build_param_space(
    &self,
    rect: NativeRect,
    ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let u_map = angular_map(rect.u, ctx)?;
    let (v_map, generatrix) = self.curve.param_axis(rect.v, ctx)?;
    Ok(revolve_param_space(&self.axis, u_map, v_map, &generatrix))
  }
}
