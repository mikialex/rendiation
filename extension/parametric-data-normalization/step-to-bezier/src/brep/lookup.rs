//! Low level table access: reference resolution, entity type names for diagnostics, points,
//! directions, placements and raw records of entities without typed holders.

use step_reader::entities::*;
use step_reader::ruststep::ast::{Name, Parameter, Record};
use step_reader::ruststep::tables::PlaceHolder;

use super::builder::*;
use super::*;

/// STEP id of a reference, `None` for inline (owned) instances.
pub(super) fn ref_id<T>(ph: &PlaceHolder<T>) -> Option<u64> {
  match ph {
    PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
    _ => None,
  }
}

/// Resolve a place holder against one step reader table. Inline instances resolve to
/// themselves.
#[allow(clippy::disallowed_types)]
pub(super) fn resolve<'t, T>(
  t: &Table,
  ph: &'t PlaceHolder<T>,
  map: &'t std::collections::HashMap<u64, T>,
  expected: &str,
) -> R<&'t T> {
  match ph {
    PlaceHolder::Owned(v) => Ok(v),
    PlaceHolder::Ref(Name::Entity(id)) => map
      .get(id)
      .ok_or_else(|| format!("#{id} is not a {expected} ({})", describe(t, *id))),
    PlaceHolder::Ref(name) => Err(format!("unsupported reference {name:?}")),
  }
}

pub(super) fn ref_or_err<T>(ph: &PlaceHolder<T>, what: &str) -> R<u64> {
  ref_id(ph).ok_or_else(|| format!("inline {what} is not supported here"))
}

/// Human readable type of an entity for diagnostics: `MISSING` when the id does not exist (or
/// failed to parse), otherwise the entity type name.
pub(super) fn describe(t: &Table, id: u64) -> String {
  type_name(t, id).unwrap_or_else(|| "MISSING".into())
}

/// Entity type name of `id` if it exists in the table.
pub(super) fn type_name(t: &Table, id: u64) -> Option<String> {
  if let Some(n) = t.subtype_name.get(&id) {
    return Some(n.clone());
  }
  if let Some(u) = t.unrecognized.get(&id) {
    return Some(u.entity_name.clone());
  }
  macro_rules! check {
    ($($field:ident => $name:literal),* $(,)?) => {
      $(if t.$field.contains_key(&id) { return Some($name.into()); })*
    };
  }
  check!(
    cartesian_point => "CARTESIAN_POINT",
    direction => "DIRECTION",
    vector => "VECTOR",
    axis1_placement => "AXIS1_PLACEMENT",
    axis2_placement_2d => "AXIS2_PLACEMENT_2D",
    axis2_placement_3d => "AXIS2_PLACEMENT_3D",
    line => "LINE",
    polyline => "POLYLINE",
    circle => "CIRCLE",
    ellipse => "ELLIPSE",
    hyperbola => "HYPERBOLA",
    parabola => "PARABOLA",
    b_spline_curve_with_knots => "B_SPLINE_CURVE_WITH_KNOTS",
    bezier_curve => "BEZIER_CURVE",
    rational_b_spline_curve => "RATIONAL_B_SPLINE_CURVE",
    trimmed_curve => "TRIMMED_CURVE",
    composite_curve => "COMPOSITE_CURVE",
    composite_curve_segment => "COMPOSITE_CURVE_SEGMENT",
    offset_curve_3d => "OFFSET_CURVE_3D",
    pcurve => "PCURVE",
    surface_curve => "SURFACE_CURVE",
    plane => "PLANE",
    spherical_surface => "SPHERICAL_SURFACE",
    cylindrical_surface => "CYLINDRICAL_SURFACE",
    toroidal_surface => "TOROIDAL_SURFACE",
    conical_surface => "CONICAL_SURFACE",
    b_spline_surface_with_knots => "B_SPLINE_SURFACE_WITH_KNOTS",
    bezier_surface => "BEZIER_SURFACE",
    rational_b_spline_surface => "RATIONAL_B_SPLINE_SURFACE",
    surface_of_linear_extrusion => "SURFACE_OF_LINEAR_EXTRUSION",
    surface_of_revolution => "SURFACE_OF_REVOLUTION",
    offset_surface => "OFFSET_SURFACE",
    vertex_point => "VERTEX_POINT",
    vertex_loop => "VERTEX_LOOP",
    edge_curve => "EDGE_CURVE",
    oriented_edge => "ORIENTED_EDGE",
    edge_loop => "EDGE_LOOP",
    face_bound => "FACE_BOUND",
    face_surface => "FACE_SURFACE",
    oriented_face => "ORIENTED_FACE",
    shell => "SHELL",
    oriented_shell => "ORIENTED_SHELL",
    shell_based_surface_model => "SHELL_BASED_SURFACE_MODEL",
    manifold_solid_brep => "MANIFOLD_SOLID_BREP",
    faceted_brep => "FACETED_BREP",
    representation => "REPRESENTATION",
    shape_representation => "SHAPE_REPRESENTATION",
    mapped_item => "MAPPED_ITEM",
    item_defined_transformation => "ITEM_DEFINED_TRANSFORMATION",
    definitional_representation => "DEFINITIONAL_REPRESENTATION",
  );
  None
}

/// Raw partial records of an entity the step reader has no typed holder for.
pub(super) fn raw_records(t: &Table, id: u64) -> Option<&[Record]> {
  t.unrecognized_records.get(&id).map(|r| r.as_slice())
}

/// Attribute list of the record named `name` of an untyped entity (simple or complex).
pub(super) fn raw_record<'t>(t: &'t Table, id: u64, name: &str) -> Option<&'t [Parameter]> {
  raw_records(t, id)?
    .iter()
    .find(|r| r.name == name)
    .and_then(|r| match &r.parameter {
      Parameter::List(p) => Some(p.as_slice()),
      _ => None,
    })
}

pub(super) fn param_ref(p: &Parameter) -> Option<u64> {
  match p {
    Parameter::Ref(Name::Entity(id)) => Some(*id),
    _ => None,
  }
}

pub(super) fn param_refs(p: &Parameter) -> Vec<u64> {
  match p {
    Parameter::List(items) => items.iter().filter_map(param_ref).collect(),
    other => param_ref(other).into_iter().collect(),
  }
}

/// A real value, possibly wrapped in a typed parameter like `LENGTH_MEASURE(1.0)`.
pub(super) fn param_real(p: &Parameter) -> Option<f64> {
  match p {
    Parameter::Real(v) => Some(*v),
    Parameter::Integer(v) => Some(*v as f64),
    Parameter::Typed { parameter, .. } => param_real(parameter),
    _ => None,
  }
}

pub(super) fn param_enum(p: &Parameter) -> Option<&str> {
  match p {
    Parameter::Enumeration(e) => Some(e.trim_matches('.')),
    _ => None,
  }
}

fn finite(v: f64, what: &str) -> R<f64> {
  if v.is_finite() {
    Ok(v)
  } else {
    Err(format!("non finite {what}"))
  }
}

impl Builder<'_> {
  pub fn point3(&self, ph: &PlaceHolder<CartesianPointHolder>) -> R<Vec3<f64>> {
    let p = resolve(self.t, ph, &self.t.cartesian_point, "CARTESIAN_POINT")?;
    match p.coordinates.as_slice() {
      [x, y, z] => Ok(Vec3::new(
        finite(*x, "coordinate")?,
        finite(*y, "coordinate")?,
        finite(*z, "coordinate")?,
      )),
      c => Err(format!(
        "{}cartesian point with {} coordinates where 3 are expected",
        ref_id(ph).map(|id| format!("#{id}: ")).unwrap_or_default(),
        c.len()
      )),
    }
  }

  pub fn point2(&self, ph: &PlaceHolder<CartesianPointHolder>) -> R<Vec2<f64>> {
    let p = resolve(self.t, ph, &self.t.cartesian_point, "CARTESIAN_POINT")?;
    match p.coordinates.as_slice() {
      [x, y] => Ok(Vec2::new(
        finite(*x, "coordinate")?,
        finite(*y, "coordinate")?,
      )),
      c => Err(format!(
        "{}cartesian point with {} coordinates where 2 are expected",
        ref_id(ph).map(|id| format!("#{id}: ")).unwrap_or_default(),
        c.len()
      )),
    }
  }

  /// Unit vector of a `DIRECTION`.
  pub fn dir3(&self, ph: &PlaceHolder<DirectionHolder>) -> R<Vec3<f64>> {
    let d = resolve(self.t, ph, &self.t.direction, "DIRECTION")?;
    let v = match d.direction_ratios.as_slice() {
      [x, y, z] => Vec3::new(*x, *y, *z),
      r => {
        return Err(format!(
          "direction with {} ratios where 3 are expected",
          r.len()
        ));
      }
    };
    let len = v.length();
    if !(len.is_finite() && len > 0.0) {
      return Err("zero or non finite direction".into());
    }
    Ok(v / len)
  }

  pub fn dir2(&self, ph: &PlaceHolder<DirectionHolder>) -> R<Vec2<f64>> {
    let d = resolve(self.t, ph, &self.t.direction, "DIRECTION")?;
    let v = match d.direction_ratios.as_slice() {
      [x, y] => Vec2::new(*x, *y),
      r => {
        return Err(format!(
          "direction with {} ratios where 2 are expected",
          r.len()
        ));
      }
    };
    let len = v.length();
    if !(len.is_finite() && len > 0.0) {
      return Err("zero or non finite direction".into());
    }
    Ok(v / len)
  }

  /// `VECTOR`: normalized orientation times magnitude.
  pub fn vector3(&self, ph: &PlaceHolder<VectorHolder>) -> R<Vec3<f64>> {
    let v = resolve(self.t, ph, &self.t.vector, "VECTOR")?;
    let magnitude = finite(v.magnitude, "vector magnitude")?;
    if magnitude <= 0.0 {
      return Err(format!("vector magnitude {magnitude} is not positive"));
    }
    Ok(self.dir3(&v.orientation)? * magnitude)
  }

  pub fn vector2(&self, ph: &PlaceHolder<VectorHolder>) -> R<Vec2<f64>> {
    let v = resolve(self.t, ph, &self.t.vector, "VECTOR")?;
    let magnitude = finite(v.magnitude, "vector magnitude")?;
    if magnitude <= 0.0 {
      return Err(format!("vector magnitude {magnitude} is not positive"));
    }
    Ok(self.dir2(&v.orientation)? * magnitude)
  }

  /// `AXIS2_PLACEMENT_3D` following ISO 10303-42 `build_axes`. A reference direction parallel
  /// to the axis violates the standard; a perpendicular direction is picked and a warning is
  /// pushed.
  pub fn frame3(&mut self, ph: &PlaceHolder<Axis2Placement3dHolder>) -> R<Frame> {
    let a = resolve(self.t, ph, &self.t.axis2_placement_3d, "AXIS2_PLACEMENT_3D")?;
    let origin = self.point3(&a.location)?;
    let z = self.dir3(&a.axis)?;
    let ref_dir = match &a.ref_direction {
      Some(r) => Some(self.dir3(r)?),
      None => None,
    };
    let (frame, degenerate) = build_axes(origin, z, ref_dir);
    if degenerate {
      let id = ref_id(ph).map(|id| format!("#{id}")).unwrap_or_default();
      self.warn(format!(
        "AXIS2_PLACEMENT_3D {id}: ref_direction parallel to axis, an arbitrary perpendicular is used"
      ));
    }
    Ok(frame)
  }

  pub fn frame3_by_id(&mut self, id: u64) -> R<Frame> {
    self.frame3(&PlaceHolder::Ref(Name::Entity(id)))
  }

  /// `AXIS2_PLACEMENT_2D`: `x` is the normalized reference direction, `y` its left normal.
  pub fn frame2_by_id(&self, id: u64) -> R<Frame2> {
    let a = self.t.axis2_placement_2d.get(&id).ok_or_else(|| {
      format!(
        "#{id} is not a AXIS2_PLACEMENT_2D ({})",
        describe(self.t, id)
      )
    })?;
    let origin = self.point2(&a.location)?;
    let x = self.dir2(&a.ref_direction)?;
    Ok(Frame2 {
      origin,
      x,
      y: Vec2::new(-x.y, x.x),
    })
  }
}

/// ISO 10303-42 `build_axes`. Returns the frame and whether an explicit reference direction was
/// parallel to the axis (which the standard forbids).
pub(super) fn build_axes(
  origin: Vec3<f64>,
  z: Vec3<f64>,
  ref_dir: Option<Vec3<f64>>,
) -> (Frame, bool) {
  // first_proj_axis: default reference direction is x, or y when the axis is along x.
  let default_ref = if z.x.abs() > 1.0 - 1e-12 {
    Vec3::new(0.0, 1.0, 0.0)
  } else {
    Vec3::new(1.0, 0.0, 0.0)
  };
  let mut degenerate = false;
  let project = |r: Vec3<f64>| {
    let x = r - z * r.dot(z);
    let len = x.length();
    (len > 1e-12).then(|| x / len)
  };
  let x = match ref_dir {
    Some(r) => project(r).unwrap_or_else(|| {
      degenerate = true;
      project(default_ref).unwrap_or(Vec3::new(0.0, 0.0, 1.0))
    }),
    None => project(default_ref).unwrap_or(Vec3::new(0.0, 0.0, 1.0)),
  };
  let y = z.cross(x);
  (Frame { origin, x, y, z }, degenerate)
}

impl Frame {
  /// Local to parent transform `p = origin + x·l.x + y·l.y + z·l.z`.
  pub fn to_mat4(&self) -> Mat4<f64> {
    Mat4::new_from_column(
      [self.x.x, self.x.y, self.x.z, 0.0],
      [self.y.x, self.y.y, self.y.z, 0.0],
      [self.z.x, self.z.y, self.z.z, 0.0],
      [self.origin.x, self.origin.y, self.origin.z, 1.0],
    )
  }

  /// Inverse of `to_mat4` (the frame is orthonormal, so the rotation inverse is its
  /// transpose).
  pub fn to_mat4_inverse(&self) -> Mat4<f64> {
    let o = self.origin;
    Mat4::new_from_column(
      [self.x.x, self.y.x, self.z.x, 0.0],
      [self.x.y, self.y.y, self.z.y, 0.0],
      [self.x.z, self.y.z, self.z.z, 0.0],
      [-self.x.dot(o), -self.y.dot(o), -self.z.dot(o), 1.0],
    )
  }
}
