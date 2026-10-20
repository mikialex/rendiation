//! Parsing helpers for entities that need more than a plain serde deserialization: B-spline
//! complex instances (order independent), implied knot vectors of the B-spline subtypes without
//! explicit knots, trimming selects, and omitted optional placement directions.

use ruststep::{
  ast::{Name, Parameter, Record},
  error::{Error, Result},
  primitive::Logical,
  tables::PlaceHolder,
};
use serde::Deserialize;

use crate::{entities::*, table::Table};

/// How the knot vector of a B-spline entity is obtained when it has no explicit knots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpliedKnots {
  /// `BEZIER_CURVE` / `BEZIER_SURFACE`: piecewise Bézier, knots `0, 1, ..., (n - 1) / d` with
  /// multiplicity `d + 1` at the ends and `d` inside.
  PiecewiseBezier,
  /// `QUASI_UNIFORM_CURVE` / `QUASI_UNIFORM_SURFACE`: knots `0, 1, ..., n - d`, multiplicity
  /// `d + 1` at the ends and `1` inside.
  QuasiUniform,
  /// `UNIFORM_CURVE` / `UNIFORM_SURFACE`: knots `-d, -d + 1, ..., n`, all multiplicity `1`.
  Uniform,
}

impl ImpliedKnots {
  pub fn knot_type(self) -> KnotType {
    match self {
      ImpliedKnots::PiecewiseBezier => KnotType::PiecewiseBezierKnots,
      ImpliedKnots::QuasiUniform => KnotType::QuasiUniformKnots,
      ImpliedKnots::Uniform => KnotType::UniformKnots,
    }
  }
}

/// Knot multiplicities and distinct knot values implied by ISO 10303-42 for `n_cp` control
/// points of degree `degree` (the sum of the multiplicities is always `n_cp + degree + 1`).
pub fn implied_knots(
  kind: ImpliedKnots,
  n_cp: usize,
  degree: usize,
) -> std::result::Result<(Vec<i64>, Vec<f64>), String> {
  if degree == 0 {
    return Err("degree 0 B-spline".into());
  }
  if n_cp < degree + 1 {
    return Err(format!(
      "{n_cp} control points are not enough for degree {degree}"
    ));
  }
  let d = degree as i64;
  Ok(match kind {
    ImpliedKnots::PiecewiseBezier => {
      if !(n_cp - 1).is_multiple_of(degree) {
        return Err(format!(
          "piecewise Bézier with {n_cp} control points is not a multiple of degree {degree}"
        ));
      }
      let segments = (n_cp - 1) / degree;
      let mut mults = vec![d; segments + 1];
      mults[0] = d + 1;
      mults[segments] = d + 1;
      (mults, (0..=segments).map(|i| i as f64).collect())
    }
    ImpliedKnots::QuasiUniform => {
      let interior = n_cp - degree - 1;
      let mut mults = vec![1; interior + 2];
      mults[0] = d + 1;
      mults[interior + 1] = d + 1;
      (mults, (0..interior + 2).map(|i| i as f64).collect())
    }
    ImpliedKnots::Uniform => {
      let count = n_cp + degree + 1;
      (
        vec![1; count],
        (0..count).map(|i| i as f64 - degree as f64).collect(),
      )
    }
  })
}

fn de_err(msg: impl Into<String>) -> Error {
  Error::DeserializeFailed(msg.into())
}

fn param<'a>(params: &'a [Parameter], i: usize, entity: &str) -> Result<&'a Parameter> {
  params
    .get(i)
    .ok_or_else(|| de_err(format!("{entity}: missing attribute {i}")))
}

fn record_params(record: &Record) -> &[Parameter] {
  match &record.parameter {
    Parameter::List(p) => p,
    _ => &[],
  }
}

/// Partial records of a complex instance, looked up by record name.
struct Records<'a>(&'a [Record]);

impl<'a> Records<'a> {
  fn get(&self, name: &str) -> Option<&'a [Parameter]> {
    self.0.iter().find(|r| r.name == name).map(record_params)
  }

  fn contains_key(&self, name: &str) -> bool {
    self.0.iter().any(|r| r.name == name)
  }
}

fn records_by_name(records: &[Record]) -> Records<'_> {
  Records(records)
}

fn joined_names(records: &[Record]) -> String {
  let mut names: Vec<_> = records.iter().map(|r| r.name.as_str()).collect();
  names.sort_unstable();
  names.join(" & ")
}

fn label_of(recs: &Records) -> String {
  recs
    .get("REPRESENTATION_ITEM")
    .and_then(|p| p.first())
    .and_then(|p| String::deserialize(p).ok())
    .unwrap_or_default()
}

/// Order independent handling of complex instances built from the `B_SPLINE_CURVE` partial
/// record (`BOUNDED_CURVE`, `B_SPLINE_CURVE_WITH_KNOTS` / `BEZIER_CURVE` / `UNIFORM_CURVE` /
/// `QUASI_UNIFORM_CURVE`, optional `RATIONAL_B_SPLINE_CURVE`, ...). Implied knots are made
/// explicit. Non rational instances go to `b_spline_curve_with_knots`, rational ones to
/// `rational_b_spline_curve`. Returns `None` if the instance is not a B-spline curve.
pub(crate) fn push_complex_bspline_curve(
  table: &mut Table,
  id: u64,
  records: &[Record],
) -> Option<Result<()>> {
  let recs = records_by_name(records);
  let bsp = recs.get("B_SPLINE_CURVE")?;
  Some((|| {
    let entity = "B_SPLINE_CURVE";
    let degree = i64::deserialize(param(bsp, 0, entity)?)?;
    let control_points_list: Vec<PlaceHolder<CartesianPointHolder>> =
      Deserialize::deserialize(param(bsp, 1, entity)?)?;
    let curve_form = Deserialize::deserialize(param(bsp, 2, entity)?)?;
    let closed_curve = Deserialize::deserialize(param(bsp, 3, entity)?)?;
    let self_intersect = Deserialize::deserialize(param(bsp, 4, entity)?)?;

    let implied = if recs.contains_key("BEZIER_CURVE") {
      Some(ImpliedKnots::PiecewiseBezier)
    } else if recs.contains_key("QUASI_UNIFORM_CURVE") {
      Some(ImpliedKnots::QuasiUniform)
    } else if recs.contains_key("UNIFORM_CURVE") {
      Some(ImpliedKnots::Uniform)
    } else {
      None
    };
    let (knot_multiplicities, knots, knot_spec) = if let Some(kind) = implied {
      let degree = usize::try_from(degree).map_err(|_| de_err("negative degree"))?;
      let (m, k) = implied_knots(kind, control_points_list.len(), degree).map_err(de_err)?;
      (m, k, kind.knot_type())
    } else {
      let entity = "B_SPLINE_CURVE_WITH_KNOTS";
      let with_knots = recs
        .get(entity)
        .ok_or_else(|| de_err("B_SPLINE_CURVE without knot subtype"))?;
      (
        Deserialize::deserialize(param(with_knots, 0, entity)?)?,
        Deserialize::deserialize(param(with_knots, 1, entity)?)?,
        Deserialize::deserialize(param(with_knots, 2, entity)?)?,
      )
    };

    let holder = BSplineCurveWithKnotsHolder {
      label: label_of(&recs),
      degree,
      control_points_list,
      curve_form,
      closed_curve,
      self_intersect,
      knot_multiplicities,
      knots,
      knot_spec,
    };
    if let Some(rational) = recs.get("RATIONAL_B_SPLINE_CURVE") {
      let entity = "RATIONAL_B_SPLINE_CURVE";
      let weights_data = Deserialize::deserialize(param(rational, 0, entity)?)?;
      table.rational_b_spline_curve.insert(
        id,
        RationalBSplineCurveHolder {
          non_rational_b_spline_curve: PlaceHolder::Owned(
            NonRationalBSplineCurveHolder::BSplineCurveWithKnots(Box::new(holder)),
          ),
          weights_data,
        },
      );
    } else {
      table.b_spline_curve_with_knots.insert(id, holder);
    }
    table.subtype_name.insert(id, joined_names(records));
    Ok(())
  })())
}

/// Surface counterpart of [`push_complex_bspline_curve`].
pub(crate) fn push_complex_bspline_surface(
  table: &mut Table,
  id: u64,
  records: &[Record],
) -> Option<Result<()>> {
  let recs = records_by_name(records);
  let bsp = recs.get("B_SPLINE_SURFACE")?;
  Some((|| {
    let entity = "B_SPLINE_SURFACE";
    let u_degree = i64::deserialize(param(bsp, 0, entity)?)?;
    let v_degree = i64::deserialize(param(bsp, 1, entity)?)?;
    let control_points_list: Vec<Vec<PlaceHolder<CartesianPointHolder>>> =
      Deserialize::deserialize(param(bsp, 2, entity)?)?;
    let surface_form = Deserialize::deserialize(param(bsp, 3, entity)?)?;
    let u_closed = Deserialize::deserialize(param(bsp, 4, entity)?)?;
    let v_closed = Deserialize::deserialize(param(bsp, 5, entity)?)?;
    let self_intersect = Deserialize::deserialize(param(bsp, 6, entity)?)?;

    let implied = if recs.contains_key("BEZIER_SURFACE") {
      Some(ImpliedKnots::PiecewiseBezier)
    } else if recs.contains_key("QUASI_UNIFORM_SURFACE") {
      Some(ImpliedKnots::QuasiUniform)
    } else if recs.contains_key("UNIFORM_SURFACE") {
      Some(ImpliedKnots::Uniform)
    } else {
      None
    };
    let (u_multiplicities, v_multiplicities, u_knots, v_knots, knot_spec) =
      if let Some(kind) = implied {
        let (nu, nv) = surface_grid_size(&control_points_list).map_err(de_err)?;
        let pu = usize::try_from(u_degree).map_err(|_| de_err("negative degree"))?;
        let pv = usize::try_from(v_degree).map_err(|_| de_err("negative degree"))?;
        let (um, uk) = implied_knots(kind, nu, pu).map_err(de_err)?;
        let (vm, vk) = implied_knots(kind, nv, pv).map_err(de_err)?;
        (um, vm, uk, vk, kind.knot_type())
      } else {
        let entity = "B_SPLINE_SURFACE_WITH_KNOTS";
        let with_knots = recs
          .get(entity)
          .ok_or_else(|| de_err("B_SPLINE_SURFACE without knot subtype"))?;
        (
          Deserialize::deserialize(param(with_knots, 0, entity)?)?,
          Deserialize::deserialize(param(with_knots, 1, entity)?)?,
          Deserialize::deserialize(param(with_knots, 2, entity)?)?,
          Deserialize::deserialize(param(with_knots, 3, entity)?)?,
          Deserialize::deserialize(param(with_knots, 4, entity)?)?,
        )
      };

    let holder = BSplineSurfaceWithKnotsHolder {
      label: label_of(&recs),
      u_degree,
      v_degree,
      control_points_list,
      surface_form,
      u_closed,
      v_closed,
      self_intersect,
      u_multiplicities,
      v_multiplicities,
      u_knots,
      v_knots,
      knot_spec,
    };
    if let Some(rational) = recs.get("RATIONAL_B_SPLINE_SURFACE") {
      let entity = "RATIONAL_B_SPLINE_SURFACE";
      let weights_data = Deserialize::deserialize(param(rational, 0, entity)?)?;
      table.rational_b_spline_surface.insert(
        id,
        RationalBSplineSurfaceHolder {
          non_rational_b_spline_surface: PlaceHolder::Owned(
            NonRationalBSplineSurfaceHolder::BSplineSurfaceWithKnots(Box::new(holder)),
          ),
          weights_data,
        },
      );
    } else {
      table.b_spline_surface_with_knots.insert(id, holder);
    }
    table.subtype_name.insert(id, joined_names(records));
    Ok(())
  })())
}

/// `(u count, v count)` of a STEP `control_points_list` (`[u][v]` layout), which must be
/// rectangular.
fn surface_grid_size<T>(cps: &[Vec<T>]) -> std::result::Result<(usize, usize), String> {
  let nv = cps.first().map(|row| row.len()).unwrap_or(0);
  if cps.iter().any(|row| row.len() != nv) {
    return Err("control point grid is not rectangular".into());
  }
  Ok((cps.len(), nv))
}

/// Simple `QUASI_UNIFORM_CURVE` / `UNIFORM_CURVE` / `BEZIER_CURVE`-like records: the
/// `B_SPLINE_CURVE` attributes with the implied knots made explicit.
pub(crate) fn implied_knot_curve(
  record: &Record,
  kind: ImpliedKnots,
) -> Result<BSplineCurveWithKnotsHolder> {
  let p = record_params(record);
  let entity = record.name.as_str();
  let degree = i64::deserialize(param(p, 1, entity)?)?;
  let control_points_list: Vec<PlaceHolder<CartesianPointHolder>> =
    Deserialize::deserialize(param(p, 2, entity)?)?;
  let degree_usize = usize::try_from(degree).map_err(|_| de_err("negative degree"))?;
  let (knot_multiplicities, knots) =
    implied_knots(kind, control_points_list.len(), degree_usize).map_err(de_err)?;
  Ok(BSplineCurveWithKnotsHolder {
    label: Deserialize::deserialize(param(p, 0, entity)?)?,
    degree,
    control_points_list,
    curve_form: Deserialize::deserialize(param(p, 3, entity)?)?,
    closed_curve: Deserialize::deserialize(param(p, 4, entity)?)?,
    self_intersect: Deserialize::deserialize(param(p, 5, entity)?)?,
    knot_multiplicities,
    knots,
    knot_spec: kind.knot_type(),
  })
}

/// Surface counterpart of [`implied_knot_curve`].
pub(crate) fn implied_knot_surface(
  record: &Record,
  kind: ImpliedKnots,
) -> Result<BSplineSurfaceWithKnotsHolder> {
  let p = record_params(record);
  let entity = record.name.as_str();
  let u_degree = i64::deserialize(param(p, 1, entity)?)?;
  let v_degree = i64::deserialize(param(p, 2, entity)?)?;
  let control_points_list: Vec<Vec<PlaceHolder<CartesianPointHolder>>> =
    Deserialize::deserialize(param(p, 3, entity)?)?;
  let (nu, nv) = surface_grid_size(&control_points_list).map_err(de_err)?;
  let pu = usize::try_from(u_degree).map_err(|_| de_err("negative degree"))?;
  let pv = usize::try_from(v_degree).map_err(|_| de_err("negative degree"))?;
  let (u_multiplicities, u_knots) = implied_knots(kind, nu, pu).map_err(de_err)?;
  let (v_multiplicities, v_knots) = implied_knots(kind, nv, pv).map_err(de_err)?;
  Ok(BSplineSurfaceWithKnotsHolder {
    label: Deserialize::deserialize(param(p, 0, entity)?)?,
    u_degree,
    v_degree,
    control_points_list,
    surface_form: Deserialize::deserialize(param(p, 4, entity)?)?,
    u_closed: Deserialize::deserialize(param(p, 5, entity)?)?,
    v_closed: Deserialize::deserialize(param(p, 6, entity)?)?,
    self_intersect: Deserialize::deserialize(param(p, 7, entity)?)?,
    u_multiplicities,
    v_multiplicities,
    u_knots,
    v_knots,
    knot_spec: kind.knot_type(),
  })
}

/// Complex instances containing a `SURFACE_CURVE` partial record (for example
/// `BOUNDED_SURFACE_CURVE`), stored in `surface_curve`.
pub(crate) fn push_complex_surface_curve(
  table: &mut Table,
  id: u64,
  records: &[Record],
) -> Option<Result<()>> {
  let recs = records_by_name(records);
  let (name, sc) = ["SURFACE_CURVE", "SEAM_CURVE", "INTERSECTION_CURVE"]
    .iter()
    .find_map(|n| recs.get(n).map(|p| (*n, p)))?;
  Some((|| {
    let holder = SurfaceCurveHolder {
      label: label_of(&recs),
      curve_3d: Deserialize::deserialize(param(sc, 0, name)?)?,
      associated_geometry: Deserialize::deserialize(param(sc, 1, name)?)?,
      master_representation: Deserialize::deserialize(param(sc, 2, name)?)?,
    };
    table.surface_curve.insert(id, holder);
    table.subtype_name.insert(id, joined_names(records));
    Ok(())
  })())
}

/// `TRIMMED_CURVE` trimming selects: `SET [1:2] OF trimming_select`, each either a cartesian
/// point reference or a `PARAMETER_VALUE(x)` typed parameter (plain reals are accepted too).
/// Unknown elements are an error rather than being dropped.
pub(crate) fn parse_trim_selects(p: &Parameter) -> Result<Vec<TrimSelect>> {
  let Parameter::List(items) = p else {
    return Err(de_err("trimming select is not a list"));
  };
  items
    .iter()
    .map(|item| match item {
      Parameter::Real(v) => Ok(TrimSelect::Value(*v)),
      Parameter::Integer(v) => Ok(TrimSelect::Value(*v as f64)),
      Parameter::Typed { keyword, parameter } if keyword == "PARAMETER_VALUE" => {
        match parameter.as_ref() {
          Parameter::Real(v) => Ok(TrimSelect::Value(*v)),
          Parameter::Integer(v) => Ok(TrimSelect::Value(*v as f64)),
          other => Err(de_err(format!("invalid PARAMETER_VALUE {other:?}"))),
        }
      }
      Parameter::Ref(Name::Entity(id)) => {
        Ok(TrimSelect::Point(PlaceHolder::Ref(Name::Entity(*id))))
      }
      other => Err(de_err(format!("invalid trimming select {other:?}"))),
    })
    .collect()
}

/// Replace an omitted (`$`) optional direction attribute by its ISO 10303-42 default, as an
/// inline `DIRECTION` so it deserializes into `PlaceHolder::Owned`.
pub(crate) fn default_direction_if_omitted(
  params: &mut [Parameter],
  index: usize,
  default: &[f64],
) {
  if let Some(p) = params.get_mut(index) {
    if matches!(p, Parameter::NotProvided | Parameter::Omitted) {
      *p = Parameter::Typed {
        keyword: "DIRECTION".into(),
        parameter: Box::new(Parameter::List(vec![
          Parameter::String(String::new()),
          Parameter::List(default.iter().map(|v| Parameter::Real(*v)).collect()),
        ])),
      };
    }
  }
}

/// Convert a STEP `LOGICAL` to `Option<bool>` (`UNKNOWN` is `None`).
pub fn logical_to_option(l: &Logical) -> Option<bool> {
  match l {
    Logical::True => Some(true),
    Logical::False => Some(false),
    Logical::Unknown => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn implied_knot_vectors() {
    let (m, k) = implied_knots(ImpliedKnots::QuasiUniform, 6, 3).unwrap();
    assert_eq!(m, vec![4, 1, 1, 4]);
    assert_eq!(k, vec![0.0, 1.0, 2.0, 3.0]);
    assert_eq!(m.iter().sum::<i64>(), 6 + 3 + 1);

    let (m, k) = implied_knots(ImpliedKnots::Uniform, 4, 2).unwrap();
    assert_eq!(m, vec![1; 7]);
    assert_eq!(k, vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0]);

    let (m, k) = implied_knots(ImpliedKnots::PiecewiseBezier, 7, 3).unwrap();
    assert_eq!(m, vec![4, 3, 4]);
    assert_eq!(k, vec![0.0, 1.0, 2.0]);

    assert!(implied_knots(ImpliedKnots::PiecewiseBezier, 6, 3).is_err());
    assert!(implied_knots(ImpliedKnots::QuasiUniform, 2, 3).is_err());
  }
}
