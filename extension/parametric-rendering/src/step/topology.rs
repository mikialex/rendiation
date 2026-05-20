use rendiation_algebra::*;
use rendiation_step_reader::entities::*;
use rendiation_step_reader::ruststep::ast::Name;
use rendiation_step_reader::ruststep::tables::{IntoOwned, PlaceHolder};
use rendiation_step_reader::table::Table;

use crate::step::StepReadError;

type Placement = (Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>);

/// A face surface together with its edge trim data, extracted from a STEP Table
/// using Holder-level traversal to preserve pcurve entity IDs.
pub struct FaceSurfaceData {
  pub surface: SurfaceAny,
  pub edges: Vec<EdgeData>,
  /// Shape-level placement (origin, x_dir, y_dir, z_dir) — only set when the
  /// containing ShapeRepresentation carries an Axis2Placement3d.
  pub placement: Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>)>,
}

/// Trim curve data for one edge of a FaceSurface.
pub struct EdgeData {
  pub curve_3d: CurveAny,
  #[allow(dead_code)]
  pub orientation: bool,
  /// 2D curve entity IDs from matching Pcurve(s). Tried in order for lossless
  /// extraction; falls back to numerical projection if empty or all fail.
  pub pcurve_entity_ids: Vec<u64>,
}

fn build_assembly_placement_map(table: &Table) -> std::collections::HashMap<u64, Placement> {
  use std::collections::{HashMap, HashSet};
  let srrwt_count = table
    .shape_representation_relationship_with_transformation
    .len();
  if srrwt_count == 0 {
    return HashMap::new();
  }
  crate::step::step_dbg!("step: assembly placement map — srrwt={srrwt_count}");

  // 1. Build SR → breps mapping from ShapeRepresentation items
  let mut sr_to_breps: HashMap<u64, Vec<u64>> = HashMap::new();
  for (sr_id, sr_holder) in &table.shape_representation {
    // Check items for brep references
    let mut breps: Vec<u64> = Vec::new();
    for item_ph in &sr_holder.items {
      if let Some(item_id) = entity_id_from_ph(item_ph) {
        if table.manifold_solid_brep.contains_key(&item_id) {
          breps.push(item_id);
        }
      }
    }
    if !breps.is_empty() {
      sr_to_breps.insert(*sr_id, breps);
    }
  }

  // 2. Build SR → parent SR mapping from ShapeRepresentationRelationship
  let mut sr_to_parent: HashMap<u64, u64> = HashMap::new();
  for (_id, srr_holder) in &table.shape_representation_relationship {
    let rep1_id = get_holder_ref_id(&srr_holder.rep_1);
    let rep2_id = get_holder_ref_id(&srr_holder.rep_2);
    if let (Some(absr_id), Some(sr_id)) = (rep1_id, rep2_id) {
      sr_to_parent.insert(absr_id, sr_id);
    }
  }

  // 3. Build transform stack from ShapeRepresentationRelationshipWithTransformation
  let mut parent_to_children: HashMap<u64, Vec<(u64, Placement)>> = HashMap::new();
  for (_id, srrwt_holder) in &table.shape_representation_relationship_with_transformation {
    let rep1_id = get_holder_ref_id(&srrwt_holder.rep_1);
    let rep2_id = get_holder_ref_id(&srrwt_holder.rep_2);
    let trans_op_id = get_holder_ref_id(&srrwt_holder.transformation_operator);
    let (Some(rep1), Some(rep2), Some(trans_id)) = (rep1_id, rep2_id, trans_op_id) else {
      continue;
    };

    let Some(matrix) = compute_idt_matrix(table, trans_id) else {
      continue;
    };

    parent_to_children
      .entry(rep2)
      .or_default()
      .push((rep1, matrix));
  }

  if parent_to_children.is_empty() {
    return HashMap::new();
  }

  // 4. Walk from roots
  let all_children: HashSet<u64> = parent_to_children
    .values()
    .flat_map(|v| v.iter().map(|(child, _)| *child))
    .collect();
  let roots: Vec<u64> = parent_to_children
    .keys()
    .filter(|k| !all_children.contains(k))
    .copied()
    .collect();

  let identity = (
    Vec3::zero(),
    Vec3::new(1., 0., 0.),
    Vec3::new(0., 1., 0.),
    Vec3::new(0., 0., 1.),
  );
  let mut sr_placement: HashMap<u64, Placement> = HashMap::new();
  for root in &roots {
    let mut stack: Vec<(u64, Placement)> = vec![(*root, identity)];
    while let Some((sr_id, parent_mat)) = stack.pop() {
      sr_placement.insert(sr_id, parent_mat);
      if let Some(children) = parent_to_children.get(&sr_id) {
        for (child_sr, child_mat) in children {
          stack.push((*child_sr, compose_placement(&parent_mat, child_mat)));
        }
      }
    }
  }

  // 5. Map SR placement to Brep placement
  crate::step::step_dbg!(
    "step: assembly placement — sr_to_breps={} sr_to_parent={} sr_placement={}",
    sr_to_breps.len(),
    sr_to_parent.len(),
    sr_placement.len()
  );
  let mut brep_placement: HashMap<u64, Placement> = HashMap::new();
  for (sr_id, brep_ids) in &sr_to_breps {
    let mut current = *sr_id;
    let mut pl = sr_placement.get(&current).copied().unwrap_or(identity);
    while let Some(parent_sr) = sr_to_parent.get(&current) {
      if *parent_sr == current {
        break;
      }
      current = *parent_sr;
      if let Some(parent_pl) = sr_placement.get(&current) {
        pl = compose_placement(&pl, parent_pl);
      }
    }
    for brep_id in brep_ids {
      brep_placement.insert(*brep_id, pl);
    }
  }
  brep_placement
}

/// Get the entity ID referenced by a PlaceHolder.
fn get_holder_ref_id<T>(ph: &PlaceHolder<T>) -> Option<u64> {
  match ph {
    PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
    _ => None,
  }
}

/// Compute the transform matrix from an ItemDefinedTransformation.
/// Returns the relative placement matrix = mat2 * mat1⁻¹.
fn compute_idt_matrix(table: &Table, idt_entity_id: u64) -> Option<Placement> {
  let idt_holder = table.item_defined_transformation.get(&idt_entity_id)?;
  let ax1_id = get_holder_ref_id(&idt_holder.transform_item_1)?;
  let ax2_id = get_holder_ref_id(&idt_holder.transform_item_2)?;
  let ax1 = table
    .axis2_placement_3d
    .get(&ax1_id)?
    .clone()
    .into_owned(table)
    .ok()?;
  let ax2 = table
    .axis2_placement_3d
    .get(&ax2_id)?
    .clone()
    .into_owned(table)
    .ok()?;
  let mat1 = axis2_placement_to_transform(&ax1);
  let mat2 = axis2_placement_to_transform(&ax2);
  Some(compose_placement(&mat2, &invert_placement(&mat1)))
}

fn invert_placement(pl: &Placement) -> Placement {
  let (origin, x, y, z) = *pl;
  let inv_origin = Vec3::new(-origin.dot(x), -origin.dot(y), -origin.dot(z));
  (
    inv_origin,
    Vec3::new(x.x, y.x, z.x),
    Vec3::new(x.y, y.y, z.y),
    Vec3::new(x.z, y.z, z.z),
  )
}

fn compose_placement(a: &Placement, b: &Placement) -> Placement {
  let (oa, xa, ya, za) = *a;
  let (ob, xb, yb, zb) = *b;
  let origin = oa + xa * ob.x + ya * ob.y + za * ob.z;
  let x = xa * xb.x + ya * xb.y + za * xb.z;
  let y = xa * yb.x + ya * yb.y + za * yb.z;
  let z = xa * zb.x + ya * zb.y + za * zb.z;
  (origin, x, y, z)
}

fn build_placement_map(table: &Table) -> std::collections::HashMap<u64, Placement> {
  use std::collections::HashMap;
  let mut map = HashMap::new();
  for (_sr_id, sr) in &table.shape_representation {
    let mut placement: Option<Placement> = None;
    let mut brep_ids: Vec<u64> = Vec::new();
    for item_ph in &sr.items {
      let item_id = match item_ph {
        PlaceHolder::Ref(Name::Entity(id)) => *id,
        _ => continue,
      };
      if let Some(ax_holder) = table.axis2_placement_3d.get(&item_id) {
        if let Ok(ax) = ax_holder.clone().into_owned(table) {
          placement = Some(axis2_placement_to_transform(&ax));
        }
      }
      if table.manifold_solid_brep.contains_key(&item_id) {
        brep_ids.push(item_id);
      }
    }
    if let Some(pl) = placement {
      for bid in brep_ids {
        map.insert(bid, pl);
      }
    }
  }
  map
}

fn axis2_placement_to_transform(ax: &Axis2Placement3d) -> Placement {
  let origin = {
    let coords = &ax.location.coordinates;
    Vec3::new(coords[0] as f32, coords[1] as f32, coords[2] as f32)
  };
  let z = {
    let d = &ax.axis.direction_ratios;
    Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize()
  };
  let x = if let Some(ref ref_dir) = ax.ref_direction {
    let d = &ref_dir.direction_ratios;
    Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize()
  } else {
    let ax = if z.x.abs() > 0.9 {
      Vec3::new(0.0, 1.0, 0.0)
    } else {
      Vec3::new(1.0, 0.0, 0.0)
    };
    z.cross(ax).cross(z).normalize()
  };
  let y = z.cross(x).normalize();
  (origin, x, y, z)
}

/// Collect face surface data from the STEP Table, preserving pcurve entity IDs
/// via Holder-level navigation.
pub fn collect_face_surface_data(table: &Table) -> Result<Vec<FaceSurfaceData>, StepReadError> {
  let assembly_placement_map = build_assembly_placement_map(table);

  crate::step::step_dbg!(
    "step: assembly entities — product_definition_shape: {}, shape_definition_representation: {}, \
     next_assembly_usage_occurrence: {}, item_defined_transformation: {}, \
     context_dependent_shape_representation: {}, shape_representation_relationship: {}, \
     shape_representation_relationship_with_transformation: {}",
    table.product_definition_shape.len(),
    table.shape_definition_representation.len(),
    table.next_assembly_usage_occurrence.len(),
    table.item_defined_transformation.len(),
    table.context_dependent_shape_representation.len(),
    table.shape_representation_relationship.len(),
    table
      .shape_representation_relationship_with_transformation
      .len(),
  );

  // Debug: dump ContextDependentShapeRepresentation items to see what they reference
  for (cdsr_id, cdsr) in &table.context_dependent_shape_representation {
    let pds_id = entity_id_from_ph(&cdsr.represented_product_relation);
    let rel_id = entity_id_from_ph(&cdsr.representation_relation);
    // Check if the relation is in shape_representation_relationship or the with_transformation table
    let in_rel = rel_id
      .map(|id| table.shape_representation_relationship.contains_key(&id))
      .unwrap_or(false);
    let in_rel_wt = rel_id
      .map(|id| {
        table
          .shape_representation_relationship_with_transformation
          .contains_key(&id)
      })
      .unwrap_or(false);
    crate::step::step_dbg!(
      "step:   cdsr #{cdsr_id} → pds={pds_id:?} rel={rel_id:?} (in_rel={in_rel}, in_rel_wt={in_rel_wt})",
    );
  }

  let placement_map = build_placement_map(table);
  let mut faces = Vec::new();

  crate::step::step_dbg!(
    "step: topology entry — manifold_solid_brep: {}, shell_based_surface_model: {}, face_surface (direct): {}",
    table.manifold_solid_brep.len(),
    table.shell_based_surface_model.len(),
    table.face_surface.len()
  );

  // Entry: ManifoldSolidBrep
  for (&brep_id, brep_holder) in &table.manifold_solid_brep {
    // Navigate: brep → outer shell → shell_elements → oriented faces → face surfaces
    if let Some(shell_id) = entity_id_from_ph(&brep_holder.outer) {
      // Prefer assembly placement (from raw STEP parsing), fall back to
      // simple ShapeRepresentation-level placement.
      let asm_pl = assembly_placement_map.get(&brep_id).copied();
      let simple_pl = placement_map.get(&brep_id).copied();
      let placement = asm_pl.or(simple_pl);
      if let (Some(asm), Some(simple)) = (asm_pl, simple_pl) {
        crate::step::step_dbg!(
          "step: brep #{brep_id}: asm origin=({:.1},{:.1},{:.1}) simple origin=({:.1},{:.1},{:.1})",
          (asm.0).x, (asm.0).y, (asm.0).z,
          (simple.0).x, (simple.0).y, (simple.0).z,
        );
      }
      crate::step::step_dbg!(
        "step: brep #{brep_id} → shell #{shell_id} placement={}",
        if placement.is_some() { "yes" } else { "no" }
      );
      collect_from_shell_id(shell_id, table, &mut faces, placement);
    } else {
      crate::step::step_dbg!("step: brep #{brep_id} outer is not a Ref");
    }
  }

  // Entry: ShellBasedSurfaceModel
  for (model_id, model_holder) in &table.shell_based_surface_model {
    crate::step::step_dbg!("step: shell_based_surface_model #{model_id}");
    for shell_ph in &model_holder.sbms_boundary {
      if let Some(shell_id) = entity_id_from_ph(shell_ph) {
        collect_from_shell_id(shell_id, table, &mut faces, None);
      }
    }
  }

  crate::step::step_dbg!(
    "step: topology result — {} FaceSurfaceData collected",
    faces.len()
  );
  Ok(faces)
}

fn collect_from_shell_id(
  shell_id: u64,
  table: &Table,
  faces: &mut Vec<FaceSurfaceData>,
  placement: Option<Placement>,
) {
  let shell = match table.shell.get(&shell_id) {
    Some(s) => s,
    None => {
      crate::step::step_dbg!("step: shell #{shell_id} not found");
      return;
    }
  };

  crate::step::step_dbg!(
    "step: shell #{shell_id} has {} elements",
    shell.shell_element.len()
  );
  for oface_ph in &shell.shell_element {
    if let Some(oface_id) = entity_id_from_ph(oface_ph) {
      // The shell element may reference an ORIENTED_FACE (stored in
      // table.oriented_face) or an ADVANCED_FACE directly (stored in
      // table.face_surface). Try both.
      if table.oriented_face.contains_key(&oface_id) {
        collect_from_oriented_face_id(oface_id, table, faces, placement);
      } else if table.face_surface.contains_key(&oface_id) {
        collect_from_face_surface_id(oface_id, table, faces, placement);
      } else {
        crate::step::step_dbg!(
          "step: element #{oface_id} not found in oriented_face or face_surface"
        );
      }
    } else {
      crate::step::step_dbg!("step: element in shell #{shell_id} is not a Ref");
    }
  }
}

fn collect_from_oriented_face_id(
  oface_id: u64,
  table: &Table,
  faces: &mut Vec<FaceSurfaceData>,
  placement: Option<Placement>,
) {
  let oface = match table.oriented_face.get(&oface_id) {
    Some(o) => o,
    None => {
      crate::step::step_dbg!("step: oriented_face #{oface_id} not found");
      return;
    }
  };

  let face_id = match entity_id_from_ph(&oface.face_element) {
    Some(id) => id,
    None => {
      crate::step::step_dbg!("step: oriented_face #{oface_id} face_element is not a Ref");
      return;
    }
  };

  let face = match table.face_surface.get(&face_id) {
    Some(f) => f,
    None => {
      crate::step::step_dbg!("step: face_surface #{face_id} not found");
      return;
    }
  };

  // Resolve the surface geometry (we need the owned SurfaceAny)
  let surface = match resolve_surface_fallback(&face.face_geometry, table) {
    Some(s) => s,
    None => {
      crate::step::step_dbg!("step: face #{face_id} surface resolution failed");
      return;
    }
  };

  // Extract edge data from bounds, preserving pcurve entity IDs
  let edges = extract_edges_from_face(&face.bounds, table);

  crate::step::step_dbg!(
    "step: face #{face_id} → {} edges ({} with pcurve)",
    edges.len(),
    edges
      .iter()
      .filter(|e| !e.pcurve_entity_ids.is_empty())
      .count()
  );

  faces.push(FaceSurfaceData {
    surface,
    edges,
    placement,
  });
}

/// Directly collect a FaceSurface when CLOSED_SHELL references ADVANCED_FACE
/// (stored in table.face_surface) rather than ORIENTED_FACE.
fn collect_from_face_surface_id(
  face_id: u64,
  table: &Table,
  faces: &mut Vec<FaceSurfaceData>,
  placement: Option<Placement>,
) {
  let face = match table.face_surface.get(&face_id) {
    Some(f) => f,
    None => {
      crate::step::step_dbg!("step: face_surface #{face_id} not found (direct)");
      return;
    }
  };

  let surface = match resolve_surface_fallback(&face.face_geometry, table) {
    Some(s) => s,
    None => {
      crate::step::step_dbg!("step: face_surface #{face_id} surface resolution failed");
      return;
    }
  };

  let edges = extract_edges_from_face(&face.bounds, table);

  crate::step::step_dbg!(
    "step: face_surface #{face_id} (direct) → {} edges ({} with pcurve)",
    edges.len(),
    edges
      .iter()
      .filter(|e| !e.pcurve_entity_ids.is_empty())
      .count()
  );

  faces.push(FaceSurfaceData {
    surface,
    edges,
    placement,
  });
}

fn extract_edges_from_face(
  bounds: &[PlaceHolder<FaceBoundHolder>],
  table: &Table,
) -> Vec<EdgeData> {
  let mut edges = Vec::new();

  crate::step::step_dbg!("step: extract_edges_from_face: {} bounds", bounds.len());

  for fb_ph in bounds {
    let fb_id = match entity_id_from_ph(fb_ph) {
      Some(id) => id,
      None => {
        crate::step::step_dbg!("step:   bound entity_id not found");
        continue;
      }
    };
    let fb = match table.face_bound.get(&fb_id) {
      Some(f) => f,
      None => {
        crate::step::step_dbg!("step:   face_bound #{fb_id} not found");
        continue;
      }
    };
    let loop_id = match entity_id_from_ph(&fb.bound) {
      Some(id) => id,
      None => {
        crate::step::step_dbg!("step:   loop entity_id not found");
        continue;
      }
    };
    let eloop = match table.edge_loop.get(&loop_id) {
      Some(e) => e,
      None => {
        crate::step::step_dbg!("step:   edge_loop #{loop_id} not found");
        continue;
      }
    };

    crate::step::step_dbg!(
      "step:   loop #{loop_id} has {} edges",
      eloop.edge_list.len()
    );

    for oe_ph in &eloop.edge_list {
      let oe_id = match entity_id_from_ph(oe_ph) {
        Some(id) => id,
        None => continue,
      };
      let oe = match table.oriented_edge.get(&oe_id) {
        Some(o) => o,
        None => {
          crate::step::step_dbg!("step:     oriented_edge #{oe_id} not found");
          continue;
        }
      };
      let orientation = oe.orientation;

      let ec_id = match entity_id_from_ph(&oe.edge_element) {
        Some(id) => id,
        None => {
          crate::step::step_dbg!("step:     edge_element entity_id not found");
          continue;
        }
      };
      let ec = match table.edge_curve.get(&ec_id) {
        Some(e) => e,
        None => {
          crate::step::step_dbg!("step:     edge_curve #{ec_id} not found");
          continue;
        }
      };

      // Resolve the 3D curve geometry directly from curve tables.
      // PlaceHolder<CurveAnyHolder>::into_owned() may not find entities
      // in all tables (e.g. surface_curve), so we do an exhaustive manual lookup.
      let curve_3d = match resolve_edge_geometry_fallback(&ec.edge_geometry, table) {
        Some(c) => c,
        None => continue,
      };

      // Extract pcurve entity IDs from the edge geometry Holder
      let pcurve_entity_ids = extract_pcurve_ids_from_edge_curve(&ec.edge_geometry, table);

      edges.push(EdgeData {
        curve_3d,
        orientation,
        pcurve_entity_ids,
      });
    }
  }

  edges
}

/// Extract pcurve 2D curve entity IDs from an edge curve Holder.
///
/// Navigates SurfaceCurve → associated_geometry → Pcurve → reference_to_curve
/// → DefinitionalRepresentation → representation_item → entity IDs.
fn extract_pcurve_ids_from_edge_curve(
  curve_ph: &PlaceHolder<CurveAnyHolder>,
  table: &Table,
) -> Vec<u64> {
  let ec_id = match entity_id_from_ph(curve_ph) {
    Some(id) => id,
    None => return Vec::new(),
  };

  // We need to check if this is a SurfaceCurve by looking up the surface_curve table directly
  let sc = match table.surface_curve.get(&ec_id) {
    Some(h) => h,
    None => return Vec::new(),
  };

  let mut ids = Vec::new();
  for assoc_ph in &sc.associated_geometry {
    let assoc_id = match entity_id_from_ph(assoc_ph) {
      Some(id) => id,
      None => continue,
    };
    let pcurve = match table.pcurve.get(&assoc_id) {
      Some(p) => p,
      None => continue,
    };

    let def_rep_id = match entity_id_from_ph(&pcurve.reference_to_curve) {
      Some(id) => id,
      None => continue,
    };
    let def_rep = match table.definitional_representation.get(&def_rep_id) {
      Some(d) => d,
      None => continue,
    };

    for item_ph in &def_rep.representation_item {
      if let Some(item_id) = entity_id_from_ph(item_ph) {
        ids.push(item_id);
      }
    }
  }
  ids
}

/// Look up a 2D curve Holder by entity ID across all curve tables.
/// Used for pcurve 2D data extraction.
pub fn find_2d_curve_holder(table: &Table, id: u64) -> Option<FoundCurveHolder<'_>> {
  if let Some(h) = table.line.get(&id) {
    return Some(FoundCurveHolder::Line(h));
  }
  if let Some(h) = table.polyline.get(&id) {
    return Some(FoundCurveHolder::Polyline(h));
  }
  if let Some(h) = table.b_spline_curve_with_knots.get(&id) {
    return Some(FoundCurveHolder::BSplineCurveWithKnots(h));
  }
  if let Some(h) = table.bezier_curve.get(&id) {
    return Some(FoundCurveHolder::BezierCurve(h));
  }
  if let Some(h) = table.rational_b_spline_curve.get(&id) {
    return Some(FoundCurveHolder::RationalBSplineCurve(h));
  }
  if let Some(h) = table.circle.get(&id) {
    return Some(FoundCurveHolder::Circle(h));
  }
  if let Some(h) = table.ellipse.get(&id) {
    return Some(FoundCurveHolder::Ellipse(h));
  }
  None
}

/// Enum referencing a curve Holder from any curve table, used for 2D extraction.
pub enum FoundCurveHolder<'a> {
  Line(&'a LineHolder),
  Polyline(&'a PolylineHolder),
  BSplineCurveWithKnots(&'a BSplineCurveWithKnotsHolder),
  BezierCurve(&'a BezierCurveHolder),
  RationalBSplineCurve(&'a RationalBSplineCurveHolder),
  Circle(&'a CircleHolder),
  Ellipse(&'a EllipseHolder),
}

// --- Helpers ---

/// Resolve edge geometry by checking ALL curve tables directly.
/// `PlaceHolder<CurveAnyHolder>::into_owned()` may not check every table
/// (e.g. surface_curve for SURFACE_CURVE entities), so we do an exhaustive
/// lookup to find the entity in whichever table it was stored.
/// Resolve a surface from `PlaceHolder<SurfaceAnyHolder>` with exhaustive
/// table lookup (the EntityTable<SurfaceAnyHolder> may not check all tables).
fn resolve_surface_fallback(
  ph: &PlaceHolder<SurfaceAnyHolder>,
  table: &Table,
) -> Option<SurfaceAny> {
  let id = entity_id_from_ph(ph)?;

  if let Some(s) = table.b_spline_surface_with_knots.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::BSplineSurfaceWithKnots(Box::new(s)));
  }
  if let Some(s) = table.bezier_surface.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::BezierSurface(Box::new(s)));
  }
  if let Some(s) = table.rational_b_spline_surface.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::RationalBSplineSurface(Box::new(s)));
  }
  if let Some(s) = table.plane.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::Plane(Box::new(s)));
  }
  if let Some(s) = table.cylindrical_surface.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::CylindricalSurface(Box::new(s)));
  }
  if let Some(s) = table.conical_surface.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::ConicalSurface(Box::new(s)));
  }
  if let Some(s) = table.spherical_surface.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::SphericalSurface(Box::new(s)));
  }
  if let Some(s) = table.toroidal_surface.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::ToroidalSurface(Box::new(s)));
  }

  None
}

fn resolve_edge_geometry_fallback(
  ph: &PlaceHolder<CurveAnyHolder>,
  table: &Table,
) -> Option<CurveAny> {
  let id = entity_id_from_ph(ph)?;
  resolve_edge_geometry_fallback_from_id(id, table)
}

fn resolve_edge_geometry_fallback_from_id(id: u64, table: &Table) -> Option<CurveAny> {
  if let Some(sc) = table.surface_curve.get(&id) {
    // Only resolve curve_3d — the associated_geometry field's PcurveOrSurface
    // entities may not be resolvable through CurveAnyHolder's EntityTable.
    let curve_id = entity_id_from_ph(&sc.curve_3d)?;
    let inner = resolve_edge_geometry_fallback_from_id(curve_id, table)?;
    return Some(CurveAny::SurfaceCurve(Box::new(SurfaceCurve {
      label: sc.label.clone(),
      curve_3d: inner,
      associated_geometry: Vec::new(),
      master_representation: sc.master_representation,
    })));
  }
  if let Some(l) = table.line.get(&id) {
    return l
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::Line(Box::new(c)));
  }
  if let Some(c) = table.circle.get(&id) {
    return c
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::Circle(Box::new(c)));
  }
  if let Some(b) = table.b_spline_curve_with_knots.get(&id) {
    return b
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::BSplineCurveWithKnots(Box::new(c)));
  }
  if let Some(b) = table.bezier_curve.get(&id) {
    return b
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::BezierCurve(Box::new(c)));
  }
  if let Some(c) = table.composite_curve.get(&id) {
    return c
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::CompositeCurve(Box::new(c)));
  }
  if let Some(t) = table.trimmed_curve.get(&id) {
    return t
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::TrimmedCurve(Box::new(c)));
  }
  if let Some(p) = table.polyline.get(&id) {
    return p
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::Polyline(Box::new(c)));
  }
  if let Some(e) = table.ellipse.get(&id) {
    return e
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::Ellipse(Box::new(c)));
  }

  None
}

fn entity_id_from_ph<T>(ph: &PlaceHolder<T>) -> Option<u64> {
  match ph {
    PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
    _ => None,
  }
}
