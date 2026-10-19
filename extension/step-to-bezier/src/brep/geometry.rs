//! Curve and surface geometry conversion to the plain f64 enums.

use rendiation_step_reader::complex::{ImpliedKnots, implied_knots, logical_to_option};
use rendiation_step_reader::entities::TrimSelect as StepTrimSelect;
use rendiation_step_reader::entities::*;
use rendiation_step_reader::ruststep::ast::Name;
use rendiation_step_reader::ruststep::tables::PlaceHolder;

use super::builder::*;
use super::lookup::*;
use super::{TrimSelect, *};

impl Builder<'_> {
  /// Dense id of a 3D curve. Unknown but existing curve types become `CurveGeom::Unsupported`;
  /// dangling references and invalid data are errors.
  pub fn curve3(&mut self, id: u64) -> R<CurveId> {
    if let Some(r) = self.curves.get(&id) {
      return r.clone();
    }
    // A surface curve used as a plain curve (composite segment, swept curve...) is its 3D curve.
    if let Some(sc) = self.t.surface_curve.get(&id) {
      let r = ref_or_err(&sc.curve_3d, "curve_3d")
        .and_then(|inner| self.guarded(id, |b| b.curve3(inner)));
      self.curves.insert(id, r.clone());
      return r;
    }
    let r = self
      .guarded(id, |b| b.convert_curve3(id))
      .map(|geom| {
        self.m.curves.push(geom);
        CurveId(self.m.curves.len() as u32 - 1)
      })
      .map_err(|e| format!("curve #{id}: {e}"));
    self.curves.insert(id, r.clone());
    r
  }

  fn convert_curve3(&mut self, id: u64) -> R<CurveGeom> {
    let t = self.t;
    if let Some(l) = t.line.get(&id) {
      return Ok(CurveGeom::Line {
        origin: self.point3(&l.pnt)?,
        vector: self.vector3(&l.dir)?,
      });
    }
    if let Some(c) = t.circle.get(&id) {
      return Ok(CurveGeom::Circle {
        frame: self.frame3(&c.position)?,
        radius: positive(c.radius, "circle radius")?,
      });
    }
    if let Some(e) = t.ellipse.get(&id) {
      return Ok(CurveGeom::Ellipse {
        frame: self.frame3(&e.position)?,
        semi_axis1: positive(e.semi_axis1, "ellipse semi axis")?,
        semi_axis2: positive(e.semi_axis2, "ellipse semi axis")?,
      });
    }
    if let Some(b) = t.b_spline_curve_with_knots.get(&id) {
      return Ok(CurveGeom::BSpline(self.bspline_curve(
        b,
        None,
        Self::point3,
      )?));
    }
    if let Some(b) = t.bezier_curve.get(&id) {
      return Ok(CurveGeom::BSpline(self.bezier_curve(
        b,
        None,
        Self::point3,
      )?));
    }
    if let Some(r) = t.rational_b_spline_curve.get(&id) {
      return Ok(CurveGeom::BSpline(self.rational_curve(r, Self::point3)?));
    }
    if let Some(p) = t.polyline.get(&id) {
      let points = p
        .points
        .iter()
        .map(|p| self.point3(p))
        .collect::<R<Vec<_>>>()?;
      if points.len() < 2 {
        return Err("polyline with less than 2 points".into());
      }
      return Ok(CurveGeom::Polyline(points));
    }
    if let Some(tc) = t.trimmed_curve.get(&id) {
      let basis = self.curve3(ref_or_err(&tc.basis_curve, "basis curve")?)?;
      let angular = matches!(
        self.m.curves[basis.index()],
        CurveGeom::Circle { .. } | CurveGeom::Ellipse { .. }
      );
      let trim1 = self.trim_selects3(&tc.trim_1, angular)?;
      let trim2 = self.trim_selects3(&tc.trim_2, angular)?;
      return Ok(CurveGeom::Trimmed {
        basis,
        trim1,
        trim2,
        sense_agreement: tc.sense_agreement,
      });
    }
    if let Some(cc) = t.composite_curve.get(&id) {
      let mut segments = Vec::with_capacity(cc.segments.len());
      for seg in &cc.segments {
        let seg = resolve(
          t,
          seg,
          &t.composite_curve_segment,
          "COMPOSITE_CURVE_SEGMENT",
        )?;
        let parent = self.curve3(ref_or_err(&seg.parent_curve, "parent curve")?)?;
        segments.push((parent, seg.same_sense));
      }
      if segments.is_empty() {
        return Err("composite curve without segments".into());
      }
      return Ok(CurveGeom::Composite(segments));
    }
    if let Some(o) = t.offset_curve_3d.get(&id) {
      return Ok(CurveGeom::Offset {
        basis: self.curve3(ref_or_err(&o.basis_curve, "basis curve")?)?,
        distance: o.distance,
        ref_direction: self.dir3(&o.ref_direction)?,
      });
    }
    if t.pcurve.contains_key(&id) || t.hyperbola.contains_key(&id) || t.parabola.contains_key(&id) {
      return Ok(unsupported_curve(t, id));
    }
    if t.unrecognized.contains_key(&id) {
      return Ok(unsupported_curve(t, id));
    }
    Err(format!("#{id} is not a curve ({})", describe(t, id)))
  }

  fn trim_selects3(&self, trims: &[StepTrimSelect], angular: bool) -> R<Vec<TrimSelect>> {
    let f = if angular { self.angle_factor } else { 1.0 };
    trims
      .iter()
      .map(|s| match s {
        StepTrimSelect::Value(v) => Ok(TrimSelect::Parameter(v * f)),
        StepTrimSelect::Point(p) => Ok(TrimSelect::Point(self.point3(p)?)),
      })
      .collect()
  }

  fn trim_selects2(
    &self,
    trims: &[StepTrimSelect],
    angular: bool,
    scale: [f64; 2],
  ) -> R<Vec<TrimSelect2>> {
    let f = if angular { self.angle_factor } else { 1.0 };
    trims
      .iter()
      .map(|s| match s {
        StepTrimSelect::Value(v) => Ok(TrimSelect2::Parameter(v * f)),
        StepTrimSelect::Point(p) => Ok(TrimSelect2::Point(scale2(self.point2(p)?, scale))),
      })
      .collect()
  }

  /// Dense id of a 2D curve (pcurve geometry). `scale` multiplies the u and v coordinates,
  /// it converts angular pcurve coordinates to radians (see [`Builder::pcurve_scale`]).
  pub fn curve2(&mut self, id: u64, scale: [f64; 2]) -> R<Curve2Id> {
    let key = (id, [scale[0].to_bits(), scale[1].to_bits()]);
    if let Some(r) = self.curves_2d.get(&key) {
      return r.clone();
    }
    let r = self
      .guarded(id, |b| b.convert_curve2(id, scale))
      .map(|geom| {
        self.m.curves_2d.push(geom);
        Curve2Id(self.m.curves_2d.len() as u32 - 1)
      })
      .map_err(|e| format!("2D curve #{id}: {e}"));
    self.curves_2d.insert(key, r.clone());
    r
  }

  fn convert_curve2(&mut self, id: u64, scale: [f64; 2]) -> R<Curve2Geom> {
    let t = self.t;
    let uniform = scale[0] == scale[1];
    if let Some(l) = t.line.get(&id) {
      return Ok(Curve2Geom::Line {
        origin: scale2(self.point2(&l.pnt)?, scale),
        vector: scale2(self.vector2(&l.dir)?, scale),
      });
    }
    if let Some(c) = t.circle.get(&id) {
      if !uniform {
        return Ok(unsupported_curve2(
          id,
          "CIRCLE in a non uniformly scaled angular pcurve",
        ));
      }
      let frame = self.frame2_by_id(ref_or_err(&c.position, "placement")?)?;
      return Ok(Curve2Geom::Circle {
        frame: scale_frame2(frame, scale[0]),
        radius: positive(c.radius, "circle radius")? * scale[0],
      });
    }
    if let Some(e) = t.ellipse.get(&id) {
      if !uniform {
        return Ok(unsupported_curve2(
          id,
          "ELLIPSE in a non uniformly scaled angular pcurve",
        ));
      }
      let frame = self.frame2_by_id(ref_or_err(&e.position, "placement")?)?;
      return Ok(Curve2Geom::Ellipse {
        frame: scale_frame2(frame, scale[0]),
        semi_axis1: positive(e.semi_axis1, "ellipse semi axis")? * scale[0],
        semi_axis2: positive(e.semi_axis2, "ellipse semi axis")? * scale[0],
      });
    }
    let point2 = |b: &Self, p: &PlaceHolder<CartesianPointHolder>| Ok(scale2(b.point2(p)?, scale));
    if let Some(b) = t.b_spline_curve_with_knots.get(&id) {
      return Ok(Curve2Geom::BSpline(self.bspline_curve(b, None, point2)?));
    }
    if let Some(b) = t.bezier_curve.get(&id) {
      return Ok(Curve2Geom::BSpline(self.bezier_curve(b, None, point2)?));
    }
    if let Some(r) = t.rational_b_spline_curve.get(&id) {
      return Ok(Curve2Geom::BSpline(self.rational_curve(r, point2)?));
    }
    if let Some(p) = t.polyline.get(&id) {
      let points = p
        .points
        .iter()
        .map(|p| point2(self, p))
        .collect::<R<Vec<_>>>()?;
      if points.len() < 2 {
        return Err("polyline with less than 2 points".into());
      }
      return Ok(Curve2Geom::Polyline(points));
    }
    if let Some(tc) = t.trimmed_curve.get(&id) {
      let basis = self.curve2(ref_or_err(&tc.basis_curve, "basis curve")?, scale)?;
      let angular = matches!(
        self.m.curves_2d[basis.index()],
        Curve2Geom::Circle { .. } | Curve2Geom::Ellipse { .. }
      );
      return Ok(Curve2Geom::Trimmed {
        basis,
        trim1: self.trim_selects2(&tc.trim_1, angular, scale)?,
        trim2: self.trim_selects2(&tc.trim_2, angular, scale)?,
        sense_agreement: tc.sense_agreement,
      });
    }
    if let Some(cc) = t.composite_curve.get(&id) {
      let mut segments = Vec::with_capacity(cc.segments.len());
      for seg in &cc.segments {
        let seg = resolve(
          t,
          seg,
          &t.composite_curve_segment,
          "COMPOSITE_CURVE_SEGMENT",
        )?;
        let parent = self.curve2(ref_or_err(&seg.parent_curve, "parent curve")?, scale)?;
        segments.push((parent, seg.same_sense));
      }
      if segments.is_empty() {
        return Err("composite curve without segments".into());
      }
      return Ok(Curve2Geom::Composite(segments));
    }
    if t.hyperbola.contains_key(&id)
      || t.parabola.contains_key(&id)
      || t.unrecognized.contains_key(&id)
    {
      let name = describe(t, id);
      return Ok(unsupported_curve2(id, &name));
    }
    Err(format!("#{id} is not a 2D curve ({})", describe(t, id)))
  }

  /// Scale to apply to pcurve coordinates on `surface`: angular native axes (cylinder, cone
  /// and revolution `u`, sphere and torus `u` and `v`) are expressed in the file plane angle
  /// unit and converted to radians, as OCCT does (`GeomConvert_Units::DegreeToRadian`).
  pub fn pcurve_scale(&self, surface: SurfaceId) -> [f64; 2] {
    let f = self.angle_factor;
    let mut s = surface;
    // Offset surfaces share the parameterization of their basis; bounded by the surface count
    // so that a malformed chain can not loop.
    for _ in 0..self.m.surfaces.len() {
      match &self.m.surfaces[s.index()] {
        SurfaceGeom::Cylinder { .. }
        | SurfaceGeom::Cone { .. }
        | SurfaceGeom::Revolution { .. } => return [f, 1.0],
        SurfaceGeom::Sphere { .. } | SurfaceGeom::Torus { .. } => return [f, f],
        SurfaceGeom::Offset { basis, .. } => s = *basis,
        _ => return [1.0, 1.0],
      }
    }
    [1.0, 1.0]
  }

  fn bspline_curve<P>(
    &self,
    b: &BSplineCurveWithKnotsHolder,
    weights: Option<&[f64]>,
    point: impl Fn(&Self, &PlaceHolder<CartesianPointHolder>) -> R<P>,
  ) -> R<BSplineCurveData<P>> {
    let degree = degree_of(b.degree)?;
    let control_points = b
      .control_points_list
      .iter()
      .map(|p| point(self, p))
      .collect::<R<Vec<_>>>()?;
    let knots = expand_knots(&b.knots, &b.knot_multiplicities, degree)?;
    check_knot_count(knots.len(), control_points.len(), degree)?;
    Ok(BSplineCurveData {
      degree,
      knots,
      weights: check_weights(weights, control_points.len())?,
      control_points,
      closed: logical_to_option(&b.closed_curve),
    })
  }

  fn bezier_curve<P>(
    &self,
    b: &BezierCurveHolder,
    weights: Option<&[f64]>,
    point: impl Fn(&Self, &PlaceHolder<CartesianPointHolder>) -> R<P>,
  ) -> R<BSplineCurveData<P>> {
    let degree = degree_of(b.degree)?;
    let control_points = b
      .control_points_list
      .iter()
      .map(|p| point(self, p))
      .collect::<R<Vec<_>>>()?;
    let (mults, distinct) =
      implied_knots(ImpliedKnots::PiecewiseBezier, control_points.len(), degree)?;
    let knots = expand_knots(&distinct, &mults, degree)?;
    Ok(BSplineCurveData {
      degree,
      knots,
      weights: check_weights(weights, control_points.len())?,
      control_points,
      closed: logical_to_option(&b.closed_curve),
    })
  }

  fn rational_curve<P>(
    &self,
    r: &RationalBSplineCurveHolder,
    point: impl Fn(&Self, &PlaceHolder<CartesianPointHolder>) -> R<P>,
  ) -> R<BSplineCurveData<P>> {
    let t = self.t;
    let weights = Some(r.weights_data.as_slice());
    let inner = match &r.non_rational_b_spline_curve {
      PlaceHolder::Owned(inner) => inner,
      PlaceHolder::Ref(Name::Entity(id)) => {
        if let Some(b) = t.b_spline_curve_with_knots.get(id) {
          return self.bspline_curve(b, weights, point);
        }
        if let Some(b) = t.bezier_curve.get(id) {
          return self.bezier_curve(b, weights, point);
        }
        return Err(format!(
          "rational curve basis #{id} is not a B-spline ({})",
          describe(t, *id)
        ));
      }
      PlaceHolder::Ref(name) => return Err(format!("unsupported reference {name:?}")),
    };
    match inner {
      NonRationalBSplineCurveHolder::BSplineCurveWithKnots(b)
      | NonRationalBSplineCurveHolder::QuasiUniformCurve(b)
      | NonRationalBSplineCurveHolder::UniformCurve(b) => self.bspline_curve(b, weights, point),
      NonRationalBSplineCurveHolder::BezierCurve(b) => self.bezier_curve(b, weights, point),
    }
  }

  /// Dense id of a surface. Unknown but existing surface types become
  /// `SurfaceGeom::Unsupported`.
  pub fn surface(&mut self, id: u64) -> R<SurfaceId> {
    if let Some(r) = self.surfaces.get(&id) {
      return r.clone();
    }
    let r = self
      .guarded(id, |b| b.convert_surface(id))
      .map(|geom| {
        self.m.surfaces.push(geom);
        SurfaceId(self.m.surfaces.len() as u32 - 1)
      })
      .map_err(|e| format!("surface #{id}: {e}"));
    self.surfaces.insert(id, r.clone());
    r
  }

  fn convert_surface(&mut self, id: u64) -> R<SurfaceGeom> {
    let t = self.t;
    if let Some(p) = t.plane.get(&id) {
      return Ok(SurfaceGeom::Plane(self.frame3(&p.position)?));
    }
    if let Some(c) = t.cylindrical_surface.get(&id) {
      return Ok(SurfaceGeom::Cylinder {
        frame: self.frame3(&c.position)?,
        radius: positive(c.radius, "cylinder radius")?,
      });
    }
    if let Some(c) = t.conical_surface.get(&id) {
      let semi_angle = c.semi_angle * self.angle_factor;
      if !(semi_angle.is_finite() && semi_angle.abs() < std::f64::consts::FRAC_PI_2) {
        return Err(format!("cone semi angle {semi_angle} rad out of range"));
      }
      if !(c.radius.is_finite() && c.radius >= 0.0) {
        return Err(format!("cone radius {} is negative", c.radius));
      }
      return Ok(SurfaceGeom::Cone {
        frame: self.frame3(&c.position)?,
        radius: c.radius,
        semi_angle,
      });
    }
    if let Some(s) = t.spherical_surface.get(&id) {
      return Ok(SurfaceGeom::Sphere {
        frame: self.frame3(&s.position)?,
        radius: positive(s.radius, "sphere radius")?,
      });
    }
    if let Some(s) = t.toroidal_surface.get(&id) {
      return Ok(SurfaceGeom::Torus {
        frame: self.frame3(&s.position)?,
        major_radius: positive(s.major_radius, "torus major radius")?,
        minor_radius: positive(s.minor_radius, "torus minor radius")?,
      });
    }
    if let Some(b) = t.b_spline_surface_with_knots.get(&id) {
      return Ok(SurfaceGeom::BSpline(self.bspline_surface(b, None)?));
    }
    if let Some(b) = t.bezier_surface.get(&id) {
      return Ok(SurfaceGeom::BSpline(self.bezier_surface(b, None)?));
    }
    if let Some(r) = t.rational_b_spline_surface.get(&id) {
      let weights = Some(r.weights_data.as_slice());
      let data = match &r.non_rational_b_spline_surface {
        PlaceHolder::Owned(
          NonRationalBSplineSurfaceHolder::BSplineSurfaceWithKnots(b)
          | NonRationalBSplineSurfaceHolder::QuasiUniformSurface(b)
          | NonRationalBSplineSurfaceHolder::UniformSurface(b),
        ) => self.bspline_surface(b, weights)?,
        PlaceHolder::Owned(NonRationalBSplineSurfaceHolder::BezierSurface(b)) => {
          self.bezier_surface(b, weights)?
        }
        PlaceHolder::Ref(Name::Entity(inner)) => {
          if let Some(b) = t.b_spline_surface_with_knots.get(inner) {
            self.bspline_surface(b, weights)?
          } else if let Some(b) = t.bezier_surface.get(inner) {
            self.bezier_surface(b, weights)?
          } else {
            return Err(format!("rational surface basis #{inner} is not a B-spline"));
          }
        }
        PlaceHolder::Ref(name) => return Err(format!("unsupported reference {name:?}")),
      };
      return Ok(SurfaceGeom::BSpline(data));
    }
    if let Some(e) = t.surface_of_linear_extrusion.get(&id) {
      return Ok(SurfaceGeom::Extrusion {
        curve: self.curve3(ref_or_err(&e.swept_curve, "swept curve")?)?,
        vector: self.vector3(&e.extrusion_axis)?,
      });
    }
    if let Some(r) = t.surface_of_revolution.get(&id) {
      let axis = resolve(t, &r.axis_position, &t.axis1_placement, "AXIS1_PLACEMENT")?;
      return Ok(SurfaceGeom::Revolution {
        curve: self.curve3(ref_or_err(&r.swept_curve, "swept curve")?)?,
        axis_origin: self.point3(&axis.location)?,
        axis_dir: self.dir3(&axis.axis)?,
      });
    }
    if let Some(o) = t.offset_surface.get(&id) {
      return Ok(SurfaceGeom::Offset {
        basis: self.surface(ref_or_err(&o.basis_surface, "basis surface")?)?,
        distance: o.distance,
      });
    }
    if t.unrecognized.contains_key(&id) {
      return Ok(SurfaceGeom::Unsupported {
        step_id: id,
        type_name: describe(t, id),
      });
    }
    Err(format!("#{id} is not a surface ({})", describe(t, id)))
  }

  fn surface_points(
    &self,
    cps: &[Vec<PlaceHolder<CartesianPointHolder>>],
  ) -> R<Vec<Vec<Vec3<f64>>>> {
    let nv = cps.first().map(|row| row.len()).unwrap_or(0);
    if cps.is_empty() || nv == 0 || cps.iter().any(|row| row.len() != nv) {
      return Err("control point grid is empty or not rectangular".into());
    }
    cps
      .iter()
      .map(|row| row.iter().map(|p| self.point3(p)).collect())
      .collect()
  }

  fn bspline_surface(
    &self,
    b: &BSplineSurfaceWithKnotsHolder,
    weights: Option<&[Vec<f64>]>,
  ) -> R<BSplineSurfaceData> {
    let degree_u = degree_of(b.u_degree)?;
    let degree_v = degree_of(b.v_degree)?;
    let control_points = self.surface_points(&b.control_points_list)?;
    let (nu, nv) = (control_points.len(), control_points[0].len());
    let knots_u = expand_knots(&b.u_knots, &b.u_multiplicities, degree_u)?;
    let knots_v = expand_knots(&b.v_knots, &b.v_multiplicities, degree_v)?;
    check_knot_count(knots_u.len(), nu, degree_u).map_err(|e| format!("u: {e}"))?;
    check_knot_count(knots_v.len(), nv, degree_v).map_err(|e| format!("v: {e}"))?;
    Ok(BSplineSurfaceData {
      degree_u,
      degree_v,
      knots_u,
      knots_v,
      weights: check_surface_weights(weights, nu, nv)?,
      control_points,
      u_closed: logical_to_option(&b.u_closed),
      v_closed: logical_to_option(&b.v_closed),
    })
  }

  fn bezier_surface(
    &self,
    b: &BezierSurfaceHolder,
    weights: Option<&[Vec<f64>]>,
  ) -> R<BSplineSurfaceData> {
    let degree_u = degree_of(b.u_degree)?;
    let degree_v = degree_of(b.v_degree)?;
    let control_points = self.surface_points(&b.control_points_list)?;
    let (nu, nv) = (control_points.len(), control_points[0].len());
    let (mu, ku) = implied_knots(ImpliedKnots::PiecewiseBezier, nu, degree_u)?;
    let (mv, kv) = implied_knots(ImpliedKnots::PiecewiseBezier, nv, degree_v)?;
    Ok(BSplineSurfaceData {
      degree_u,
      degree_v,
      knots_u: expand_knots(&ku, &mu, degree_u)?,
      knots_v: expand_knots(&kv, &mv, degree_v)?,
      weights: check_surface_weights(weights, nu, nv)?,
      control_points,
      u_closed: logical_to_option(&b.u_closed),
      v_closed: logical_to_option(&b.v_closed),
    })
  }
}

fn unsupported_curve(t: &Table, id: u64) -> CurveGeom {
  CurveGeom::Unsupported {
    step_id: id,
    type_name: describe(t, id),
  }
}

fn unsupported_curve2(id: u64, type_name: &str) -> Curve2Geom {
  Curve2Geom::Unsupported {
    step_id: id,
    type_name: type_name.into(),
  }
}

fn scale2(p: Vec2<f64>, s: [f64; 2]) -> Vec2<f64> {
  Vec2::new(p.x * s[0], p.y * s[1])
}

fn scale_frame2(f: Frame2, s: f64) -> Frame2 {
  Frame2 {
    origin: f.origin * s,
    ..f
  }
}

fn positive(v: f64, what: &str) -> R<f64> {
  if v.is_finite() && v > 0.0 {
    Ok(v)
  } else {
    Err(format!("{what} {v} is not positive"))
  }
}

fn degree_of(d: i64) -> R<usize> {
  match usize::try_from(d) {
    Ok(d) if d >= 1 => Ok(d),
    _ => Err(format!("invalid B-spline degree {d}")),
  }
}

/// Expand distinct knots with their multiplicities. Knots must be strictly increasing and every
/// multiplicity in `1..=degree + 1`.
pub(super) fn expand_knots(knots: &[f64], mults: &[i64], degree: usize) -> R<Vec<f64>> {
  if knots.len() != mults.len() {
    return Err(format!(
      "{} knots but {} multiplicities",
      knots.len(),
      mults.len()
    ));
  }
  if knots.len() < 2 {
    return Err("less than 2 distinct knots".into());
  }
  let mut out = Vec::new();
  for (i, (k, m)) in knots.iter().zip(mults).enumerate() {
    if !k.is_finite() {
      return Err("non finite knot".into());
    }
    if i > 0 && *k <= knots[i - 1] {
      return Err(format!("knots not strictly increasing at index {i}"));
    }
    let m = usize::try_from(*m)
      .ok()
      .filter(|m| (1..=degree + 1).contains(m))
      .ok_or_else(|| format!("invalid knot multiplicity {m} for degree {degree}"))?;
    out.extend(std::iter::repeat_n(*k, m));
  }
  Ok(out)
}

fn check_knot_count(knots: usize, control_points: usize, degree: usize) -> R<()> {
  if control_points < degree + 1 {
    return Err(format!(
      "{control_points} control points are not enough for degree {degree}"
    ));
  }
  if knots != control_points + degree + 1 {
    return Err(format!(
      "{knots} knots (expanded) but {control_points} control points of degree {degree} need {}",
      control_points + degree + 1
    ));
  }
  Ok(())
}

fn check_weights(weights: Option<&[f64]>, n: usize) -> R<Option<Vec<f64>>> {
  let Some(w) = weights else {
    return Ok(None);
  };
  if w.len() != n {
    return Err(format!("{} weights for {n} control points", w.len()));
  }
  if w.iter().any(|w| !(w.is_finite() && *w > 0.0)) {
    return Err("non positive weight".into());
  }
  Ok(Some(w.to_vec()))
}

fn check_surface_weights(
  weights: Option<&[Vec<f64>]>,
  nu: usize,
  nv: usize,
) -> R<Option<Vec<Vec<f64>>>> {
  let Some(w) = weights else {
    return Ok(None);
  };
  if w.len() != nu || w.iter().any(|row| row.len() != nv) {
    return Err(format!(
      "weight grid does not match the {nu}x{nv} control points"
    ));
  }
  if w.iter().flatten().any(|w| !(w.is_finite() && *w > 0.0)) {
    return Err("non positive weight".into());
  }
  Ok(Some(w.to_vec()))
}
