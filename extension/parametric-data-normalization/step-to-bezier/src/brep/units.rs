//! Units and tolerance from the `GEOMETRIC_REPRESENTATION_CONTEXT` complex instances
//! (`GLOBAL_UNIT_ASSIGNED_CONTEXT`, `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT`).

use step_reader::ruststep::ast::Parameter;

use super::builder::*;
use super::lookup::*;
use super::*;

#[derive(Debug, Clone, Copy, Default)]
struct ContextUnits {
  length_mm: Option<f64>,
  angle_rad: Option<f64>,
  /// Largest length uncertainty, in millimetres.
  uncertainty_mm: Option<f64>,
}

#[derive(Clone, Copy, PartialEq)]
enum UnitKind {
  Length,
  Angle,
}

impl Builder<'_> {
  /// Fill `length_unit_mm`, `plane_angle_unit_rad` and `uncertainty`. The contexts of the
  /// representations holding bodies are used; when there is none, every context with assigned
  /// units. Disagreeing units are reported, the first context (by STEP id) wins; the
  /// uncertainty is the largest one found.
  pub fn resolve_units(&mut self) {
    let t = self.t;
    let mut contexts: Vec<_> = self
      .representations()
      .values()
      .filter(|rep| rep.items.iter().any(|i| is_body(t, *i)))
      .filter_map(|rep| rep.context)
      .filter(|ctx| raw_record(t, *ctx, "GLOBAL_UNIT_ASSIGNED_CONTEXT").is_some())
      .collect();
    if contexts.is_empty() {
      contexts = t
        .unrecognized_records
        .iter()
        .filter(|(_, records)| {
          records
            .iter()
            .any(|r| r.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
        })
        .map(|(id, _)| *id)
        .collect();
    }
    contexts.sort_unstable();
    contexts.dedup();

    let mut length: Option<(u64, f64)> = None;
    let mut angle: Option<(u64, f64)> = None;
    let mut uncertainty_mm: Option<f64> = None;
    for ctx in contexts {
      let units = context_units(t, ctx);
      if let Some(mm) = units.length_mm {
        match length {
          None => length = Some((ctx, mm)),
          Some((first, v)) if !same(v, mm) => self.warn(format!(
            "context #{ctx} length unit {mm} mm differs from context #{first} ({v} mm), using the latter"
          )),
          _ => {}
        }
      }
      if let Some(rad) = units.angle_rad {
        match angle {
          None => angle = Some((ctx, rad)),
          Some((first, v)) if !same(v, rad) => self.warn(format!(
            "context #{ctx} plane angle unit {rad} rad differs from context #{first} ({v} rad), using the latter"
          )),
          _ => {}
        }
      }
      if let Some(u) = units.uncertainty_mm {
        uncertainty_mm = Some(uncertainty_mm.map_or(u, |v| v.max(u)));
      }
    }
    self.m.length_unit_mm = length.map(|(_, v)| v);
    self.m.plane_angle_unit_rad = angle.map(|(_, v)| v);
    self.m.uncertainty = match (uncertainty_mm, self.m.length_unit_mm) {
      (Some(u), Some(mm)) => Some(u / mm),
      // Without a length unit the uncertainty was expressed in its own (unknown) unit.
      (Some(u), None) => Some(u),
      _ => None,
    };
  }
}

fn same(a: f64, b: f64) -> bool {
  (a - b).abs() <= 1e-9 * a.abs().max(b.abs())
}

pub(super) fn is_body(t: &Table, id: u64) -> bool {
  t.manifold_solid_brep.contains_key(&id)
    || t.shell_based_surface_model.contains_key(&id)
    || t.faceted_brep.contains_key(&id)
}

fn context_units(t: &Table, ctx: u64) -> ContextUnits {
  let mut out = ContextUnits::default();
  if let Some(p) = raw_record(t, ctx, "GLOBAL_UNIT_ASSIGNED_CONTEXT") {
    for unit in p.first().map(param_refs).unwrap_or_default() {
      match unit_kind(t, unit) {
        Some(UnitKind::Length) => out.length_mm = unit_scale(t, unit, 0),
        Some(UnitKind::Angle) => out.angle_rad = unit_scale(t, unit, 0),
        None => {}
      }
    }
  }
  if let Some(p) = raw_record(t, ctx, "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT") {
    for u in p.first().map(param_refs).unwrap_or_default() {
      let Some(m) = raw_record(t, u, "UNCERTAINTY_MEASURE_WITH_UNIT") else {
        continue;
      };
      let (Some(value), Some(unit)) = (m.first(), m.get(1).and_then(param_ref)) else {
        continue;
      };
      // Angular uncertainties (SOLID_ANGLE_MEASURE...) are ignored.
      if unit_kind(t, unit) != Some(UnitKind::Length) {
        continue;
      }
      if let (Some(v), Some(mm)) = (param_real(value), unit_scale(t, unit, 0)) {
        let v = v.abs() * mm;
        out.uncertainty_mm = Some(out.uncertainty_mm.map_or(v, |o| o.max(v)));
      }
    }
  }
  out
}

fn unit_kind(t: &Table, unit: u64) -> Option<UnitKind> {
  let records = raw_records(t, unit)?;
  if records.iter().any(|r| r.name == "LENGTH_UNIT") {
    Some(UnitKind::Length)
  } else if records.iter().any(|r| r.name == "PLANE_ANGLE_UNIT") {
    Some(UnitKind::Angle)
  } else {
    None
  }
}

/// Size of a unit: millimetres for lengths, radians for plane angles.
fn unit_scale(t: &Table, unit: u64, depth: usize) -> Option<f64> {
  if depth > 8 {
    return None;
  }
  if let Some(si) = raw_record(t, unit, "SI_UNIT") {
    let prefix = match si.first() {
      Some(Parameter::Enumeration(p)) => si_prefix(p.trim_matches('.'))?,
      _ => 1.0,
    };
    let base = match si.get(1).and_then(param_enum)? {
      "METRE" => 1000.0,
      "RADIAN" => 1.0,
      _ => return None,
    };
    return Some(prefix * base);
  }
  if let Some(cb) = raw_record(t, unit, "CONVERSION_BASED_UNIT") {
    let measure = cb.get(1).and_then(param_ref)?;
    let records = raw_records(t, measure)?;
    let (value, base) = records.iter().find_map(|r| {
      let Parameter::List(p) = &r.parameter else {
        return None;
      };
      if !r.name.ends_with("MEASURE_WITH_UNIT") || p.len() < 2 {
        return None;
      }
      Some((param_real(&p[0])?, param_ref(&p[1])?))
    })?;
    return Some(value * unit_scale(t, base, depth + 1)?);
  }
  None
}

fn si_prefix(p: &str) -> Option<f64> {
  Some(match p {
    "EXA" => 1e18,
    "PETA" => 1e15,
    "TERA" => 1e12,
    "GIGA" => 1e9,
    "MEGA" => 1e6,
    "KILO" => 1e3,
    "HECTO" => 1e2,
    "DECA" => 1e1,
    "DECI" => 1e-1,
    "CENTI" => 1e-2,
    "MILLI" => 1e-3,
    "MICRO" => 1e-6,
    "NANO" => 1e-9,
    "PICO" => 1e-12,
    "FEMTO" => 1e-15,
    "ATTO" => 1e-18,
    _ => return None,
  })
}
