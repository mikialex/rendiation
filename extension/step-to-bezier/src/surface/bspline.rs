//! `B_SPLINE_SURFACE_WITH_KNOTS` and its rational variant.

use super::*;

/// Native parameters are the knot parameters. Closed axes (both boundary iso-curves coincide
/// within `ctx.tol`) are periodic with the domain length as period, boundary iso-curves that
/// degenerate to a point are poles.
///
/// The P space map of an axis is fixed: one affine segment per non empty knot span of the
/// whole domain, P = 0 at the domain start. `build_param_space` therefore requires periodic
/// coordinates of the rect to start inside the base period (see `fixed_map_cells`).
pub struct BSplineSurface {
  nurbs: NurbsSurface,
  /// Exact Bézier patches `[v_span][u_span]`.
  patches: Vec<Vec<RationalBezierSurface>>,
  maps: [PiecewiseParamMap; 2],
  domain: [(f64, f64); 2],
  poles: Vec<Pole>,
  tol: f64,
}

impl BSplineSurface {
  pub fn new(data: &BSplineSurfaceData, ctx: &ConvertCtx) -> ConvertResult<Self> {
    let count_u = data.control_points.len();
    let count_v = data.control_points.first().map_or(0, |r| r.len());
    if count_u == 0 || count_v == 0 || data.control_points.iter().any(|r| r.len() != count_v) {
      return Err(ConvertError::invalid(
        None,
        "B-spline surface control points do not form a grid",
      ));
    }
    if let Some(w) = &data.weights
      && (w.len() != count_u || w.iter().any(|r| r.len() != count_v))
    {
      return Err(ConvertError::invalid(
        None,
        "B-spline surface weights do not match the control point grid",
      ));
    }
    let mut cp = Vec::with_capacity(count_u * count_v);
    for v in 0..count_v {
      for u in 0..count_u {
        let w = data.weights.as_ref().map_or(1., |w| w[u][v]);
        cp.push(Vec4::from_euclid(data.control_points[u][v], w));
      }
    }
    let nurbs = NurbsSurface::new(
      data.degree_u,
      data.degree_v,
      data.knots_u.clone(),
      data.knots_v.clone(),
      count_u,
      count_v,
      cp,
    )?;
    let patches = nurbs.to_bezier_patches();
    let domain = nurbs.domain();
    let poles = detect_poles(&patches, domain, ctx.tol);
    let maps = [Axis::U, Axis::V].map(|axis| {
      let breaks = nurbs.span_breaks(axis);
      let closed = is_closed(&patches, axis, ctx.tol) && !poles.iter().any(|p| p.axis == axis);
      let (lo, hi) = domain[axis.index()];
      let n = breaks.len() - 1;
      PiecewiseParamMap::new(
        breaks,
        vec![SegmentMap::Affine; n],
        closed.then_some(hi - lo),
      )
    });
    Ok(Self {
      nurbs,
      patches,
      maps,
      domain,
      poles,
      tol: ctx.tol,
    })
  }

  /// Periodic coordinates reduced into the base period, so evaluation never extrapolates.
  fn reduce(&self, native: Vec2<f64>) -> Vec2<f64> {
    let r = |x: f64, axis: usize| match self.maps[axis].period {
      Some(period) => {
        let lo = self.domain[axis].0;
        lo + (x - lo).rem_euclid(period)
      }
      None => x,
    };
    Vec2::new(r(native.x, 0), r(native.y, 1))
  }

  fn search_axes(&self) -> SearchAxes {
    let axis = |i: usize| match self.maps[i].period {
      Some(period) => (None, Some((self.domain[i].0, period))),
      None => (Some(self.domain[i]), None),
    };
    let (u, v) = (axis(0), axis(1));
    SearchAxes {
      bounds: [u.0, v.0],
      periodic: [u.1, v.1],
    }
  }
}

/// Both boundary iso-curves of `axis` coincide within `tol` (sampled on every span).
fn is_closed(patches: &[Vec<RationalBezierSurface>], axis: Axis, tol: f64) -> bool {
  let pairs: Vec<_> = match axis {
    Axis::U => patches
      .iter()
      .filter_map(|row| Some((row.first()?, row.last()?)))
      .collect(),
    Axis::V => match (patches.first(), patches.last()) {
      (Some(first), Some(last)) => first.iter().zip(last).collect(),
      _ => return false,
    },
  };
  pairs.iter().all(|(a, b)| {
    let (ca, cb) = (a.iso_curve(axis, 0.), b.iso_curve(axis, 1.));
    (0..=SAMPLES).all(|k| {
      let t = k as f64 / SAMPLES as f64;
      (ca.eval(t) - cb.eval(t)).length() <= tol
    })
  })
}

const SAMPLES: usize = 6;

/// Domain boundary iso-curves whose Bézier control points (exact for unclamped knots too)
/// all coincide within `tol`.
fn detect_poles(
  patches: &[Vec<RationalBezierSurface>],
  domain: [(f64, f64); 2],
  tol: f64,
) -> Vec<Pole> {
  let mut poles = Vec::new();
  let boundary = |axis: Axis, high: bool| -> Vec<Vec3<f64>> {
    let mut points = Vec::new();
    match axis {
      Axis::U => {
        for row in patches {
          let patch = if high { row.last() } else { row.first() };
          if let Some(patch) = patch {
            let i = if high { patch.degree_u } else { 0 };
            points.extend((0..=patch.degree_v).map(|j| patch.control_point(i, j).to_euclid()));
          }
        }
      }
      Axis::V => {
        let row = if high {
          patches.last()
        } else {
          patches.first()
        };
        for patch in row.into_iter().flatten() {
          let j = if high { patch.degree_v } else { 0 };
          points.extend((0..=patch.degree_u).map(|i| patch.control_point(i, j).to_euclid()));
        }
      }
    }
    points
  };
  for axis in [Axis::U, Axis::V] {
    for high in [false, true] {
      let points = boundary(axis, high);
      let Some(first) = points.first() else {
        continue;
      };
      if points.iter().all(|p| (*p - *first).length() <= tol) {
        let (lo, hi) = domain[axis.index()];
        poles.push(Pole {
          axis,
          value: if high { hi } else { lo },
          position: *first,
        });
      }
    }
  }
  poles
}

impl SurfaceModel for BSplineSurface {
  fn kind_name(&self) -> &'static str {
    "BSpline"
  }

  fn eval(&self, native: Vec2<f64>) -> Vec3<f64> {
    let n = self.reduce(native);
    self.nurbs.eval(n.x, n.y)
  }

  fn eval_with_partials(&self, native: Vec2<f64>) -> (Vec3<f64>, Vec3<f64>, Vec3<f64>) {
    let n = self.reduce(native);
    self.nurbs.eval_with_partials(n.x, n.y)
  }

  fn period(&self) -> [Option<f64>; 2] {
    [self.maps[0].period, self.maps[1].period]
  }

  fn natural_domain(&self) -> [Option<(f64, f64)>; 2] {
    [0, 1].map(|i| self.maps[i].period.is_none().then_some(self.domain[i]))
  }

  fn period_start(&self) -> [Option<f64>; 2] {
    [0, 1].map(|i| self.maps[i].period.map(|_| self.domain[i].0))
  }

  fn poles(&self) -> Vec<Pole> {
    self.poles.clone()
  }

  fn project(&self, p: Vec3<f64>, guess: Option<Vec2<f64>>) -> Option<SurfaceProjection> {
    project_numerically(self, self.search_axes(), self.tol, p, guess, || {
      let flat: Vec<_> = self.patches.iter().flatten().collect();
      let nu = self.patches.first().map_or(0, |r| r.len());
      let (bu, bv) = (&self.maps[0].breaks, &self.maps[1].breaks);
      patch_seeds(&flat, p, SEEDS, |k, ab| {
        let (j, i) = (k / nu, k % nu);
        Vec2::new(
          bu[i] + (bu[i + 1] - bu[i]) * ab.x,
          bv[j] + (bv[j + 1] - bv[j]) * ab.y,
        )
      })
    })
  }

  /// Precondition: periodic coordinates of `rect` start inside the base period
  /// `[domain start, domain start + period]`, non periodic ones lie inside the knot domain.
  fn build_param_space(
    &self,
    rect: NativeRect,
    _ctx: &ConvertCtx,
  ) -> ConvertResult<SurfaceParamSpace> {
    let cells_u = fixed_map_cells(&self.maps[0], rect.u)?;
    let cells_v = fixed_map_cells(&self.maps[1], rect.v)?;
    let mut patches = Vec::with_capacity(cells_u.len() * cells_v.len());
    for &j in &cells_v {
      for &i in &cells_u {
        let su = segment_of_cell(&self.maps[0], i as i64);
        let sv = segment_of_cell(&self.maps[1], j as i64);
        patches.push(CellPatch {
          cell: (i, j),
          surface: self.patches[sv][su].clone(),
        });
      }
    }
    Ok(SurfaceParamSpace {
      u_map: self.maps[0].clone(),
      v_map: self.maps[1].clone(),
      patches,
    })
  }
}

const SEEDS: usize = 4;
