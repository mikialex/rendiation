use rendiation_algebra::*;
use rendiation_step_reader::entities::*;
use rendiation_step_reader::ruststep::ast::Name;
use rendiation_step_reader::ruststep::tables::{IntoOwned, PlaceHolder};
use rendiation_step_reader::table::Table;

use super::*;

/// A pcurve 2D curve entity ID paired with the surface entity on which
/// the pcurve is defined (pcurve.basis_surface). Used to filter pcurves
/// to only those belonging to the face being processed.
#[derive(Debug, Clone)]
pub struct PcurveRef {
  /// Entity ID of the 2D curve within the pcurve's definitional_representation.
  pub curve_2d_id: u64,
  /// Entity ID of the surface this pcurve is defined on (pcurve.basis_surface).
  pub surface_id: u64,
}

/// A face surface together with its edge trim data, extracted from a STEP Table
/// using Holder-level traversal to preserve pcurve entity IDs.
pub struct FaceSurfaceData {
  pub surface: SurfaceAny,
  /// Edge loops, one per FaceBound. Edges within each loop are in traversal
  /// order as defined by the STEP EdgeLoop entity.
  pub edge_loops: Vec<Vec<EdgeData>>,
  /// Shape-level placement (origin, x_dir, y_dir, z_dir) — only set when the
  /// containing ShapeRepresentation carries an Axis2Placement3d.
  pub placement: Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>, Vec3<f32>)>,
  /// Whether to flip the normal relative to du×dv.
  /// Computed from FaceSurface.same_sense and OrientedFace.orientation.
  pub is_back_face: bool,
  /// Entity ID of the face's surface geometry (from face.face_geometry).
  /// Used to match pcurves to the correct face.
  pub surface_entity_id: u64,
  /// STEP entity ID of the FaceSurface / AdvancedFace.
  pub face_id: u64,
  /// STEP entity provenance for the placement transform.
  pub placement_source: crate::PlacementSource,
}

/// Trim curve data for one edge of a FaceSurface.
pub struct EdgeData {
  pub curve_3d: CurveAny,
  /// OrientedEdge.orientation — true if the edge is traversed in the same
  /// direction as the underlying EdgeCurve.
  pub orientation: bool,
  /// EdgeCurve.same_sense — true if the curve parameterisation agrees with
  /// the edge direction (start vertex → end vertex).
  pub same_sense: bool,
  /// Pcurve references from matching Pcurve(s), each paired with its
  /// basis_surface entity ID. Length ≤ 2 (one per face sharing the edge).
  /// Tried in order; falls back to numerical projection if empty or all
  /// filtered out by surface match.
  pub pcurve_refs: Vec<PcurveRef>,
}

/// Collect face surface data from the STEP Table, preserving pcurve entity IDs
/// via Holder-level navigation.
pub fn collect_face_surface_data(table: &Table) -> Vec<FaceSurfaceData> {
  let assembly_placement_map = build_assembly_placement_map(table);

  step_dbg!(
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
    step_dbg!(
      "step:   cdsr #{cdsr_id} → pds={pds_id:?} rel={rel_id:?} (in_rel={in_rel}, in_rel_wt={in_rel_wt})",
    );
  }

  let placement_map = build_placement_map(table);
  let mut faces = Vec::new();

  step_dbg!(
    "step: topology entry — manifold_solid_brep: {}, shell_based_surface_model: {}, face_surface (direct): {}",
    table.manifold_solid_brep.len(),
    table.shell_based_surface_model.len(),
    table.face_surface.len()
  );

  // Entry: ManifoldSolidBrep
  for (&brep_id, brep_holder) in &table.manifold_solid_brep {
    // Navigate: brep → outer shell → shell_elements → oriented faces → face surfaces
    if let Some(shell_id) = entity_id_from_ph(&brep_holder.outer) {
      // Prefer assembly occurrences, fall back to simple
      // ShapeRepresentation-level placement.
      let asm_placements = assembly_placement_map.get(&brep_id);
      let simple_pl = placement_map.get(&brep_id);
      if let (Some(asm), Some(simple)) = (asm_placements.and_then(|p| p.first()), simple_pl) {
        step_dbg!(
          "step: brep #{brep_id}: asm origin=({:.1},{:.1},{:.1}) simple origin=({:.1},{:.1},{:.1})",
          (asm.placement.0).x,
          (asm.placement.0).y,
          (asm.placement.0).z,
          (simple.placement.0).x,
          (simple.placement.0).y,
          (simple.placement.0).z,
        );
      }

      if let Some(placements) = asm_placements {
        step_dbg!(
          "step: brep #{brep_id} → shell #{shell_id} assembly occurrences={}",
          placements.len()
        );
        for asm_pl in placements {
          collect_from_shell_id(
            shell_id,
            table,
            &mut faces,
            Some(asm_pl.placement),
            crate::PlacementSource::Assembly {
              brep_id,
              chain: asm_pl.chain.clone(),
            },
          );
        }
      } else if let Some(sp) = simple_pl {
        step_dbg!("step: brep #{brep_id} → shell #{shell_id} placement=yes");
        collect_from_shell_id(
          shell_id,
          table,
          &mut faces,
          Some(sp.placement),
          crate::PlacementSource::Axis2Placement3d {
            axis_id: sp.axis_id,
            shape_representation_id: sp.shape_rep_id,
          },
        );
      } else {
        step_dbg!("step: brep #{brep_id} → shell #{shell_id} placement=no");
        collect_from_shell_id(
          shell_id,
          table,
          &mut faces,
          None,
          crate::PlacementSource::Identity,
        );
      }
    } else {
      step_dbg!("step: brep #{brep_id} outer is not a Ref");
    }
  }

  // Entry: ShellBasedSurfaceModel
  for (model_id, model_holder) in &table.shell_based_surface_model {
    step_dbg!("step: shell_based_surface_model #{model_id}");
    for shell_ph in &model_holder.sbms_boundary {
      if let Some(shell_id) = entity_id_from_ph(shell_ph) {
        collect_from_shell_id(
          shell_id,
          table,
          &mut faces,
          None,
          crate::PlacementSource::Identity,
        );
      }
    }
  }

  step_dbg!(
    "step: topology result — {} FaceSurfaceData collected",
    faces.len()
  );
  faces
}

fn collect_from_shell_id(
  shell_id: u64,
  table: &Table,
  faces: &mut Vec<FaceSurfaceData>,
  placement: Option<Placement>,
  placement_source: crate::PlacementSource,
) {
  let shell = match table.shell.get(&shell_id) {
    Some(s) => s,
    None => {
      step_dbg!("step: shell #{shell_id} not found");
      return;
    }
  };

  step_dbg!(
    "step: shell #{shell_id} has {} elements",
    shell.shell_element.len()
  );
  for oface_ph in &shell.shell_element {
    if let Some(oface_id) = entity_id_from_ph(oface_ph) {
      // The shell element may reference an ORIENTED_FACE (stored in
      // table.oriented_face) or an ADVANCED_FACE directly (stored in
      // table.face_surface). Try both.
      if table.oriented_face.contains_key(&oface_id) {
        collect_from_oriented_face_id(oface_id, table, faces, placement, placement_source.clone());
      } else if table.face_surface.contains_key(&oface_id) {
        collect_from_face_surface_id(oface_id, table, faces, placement, placement_source.clone());
      } else {
        step_dbg!("step: element #{oface_id} not found in oriented_face or face_surface");
      }
    } else {
      step_dbg!("step: element in shell #{shell_id} is not a Ref");
    }
  }
}

fn collect_from_oriented_face_id(
  oface_id: u64,
  table: &Table,
  faces: &mut Vec<FaceSurfaceData>,
  placement: Option<Placement>,
  placement_source: crate::PlacementSource,
) {
  let oface = match table.oriented_face.get(&oface_id) {
    Some(o) => o,
    None => {
      step_dbg!("step: oriented_face #{oface_id} not found");
      return;
    }
  };

  let face_id = entity_id_from_ph(&oface.face_element).unwrap();

  let face = match table.face_surface.get(&face_id) {
    Some(f) => f,
    None => {
      step_dbg!("step: face_surface #{face_id} not found");
      return;
    }
  };

  let surface_entity_id = entity_id_from_ph(&face.face_geometry).unwrap();

  // Resolve the surface geometry (we need the owned SurfaceAny)
  let surface = match resolve_surface_fallback(&face.face_geometry, table) {
    Some(s) => s,
    None => {
      step_dbg!("step: face #{face_id} surface resolution failed");
      return;
    }
  };

  // Extract edge data from bounds, preserving pcurve entity IDs
  let edge_loops = extract_edges_from_face(&face.bounds, table);
  let total_edges: usize = edge_loops.iter().map(|l| l.len()).sum();

  step_dbg!(
    "step: face #{face_id} → {} loops, {} edges ({} with pcurve)",
    edge_loops.len(),
    total_edges,
    edge_loops
      .iter()
      .flat_map(|l| l.iter())
      .filter(|e| !e.pcurve_refs.is_empty())
      .count()
  );

  // Compute net flip: FaceSurface.same_sense XOR OrientedFace.orientation.
  let is_back_face = face.same_sense != oface.orientation;

  faces.push(FaceSurfaceData {
    surface,
    edge_loops,
    placement,
    is_back_face,
    surface_entity_id,
    face_id,
    placement_source,
  });
}

/// Directly collect a FaceSurface when CLOSED_SHELL references ADVANCED_FACE
/// (stored in table.face_surface) rather than ORIENTED_FACE.
fn collect_from_face_surface_id(
  face_id: u64,
  table: &Table,
  faces: &mut Vec<FaceSurfaceData>,
  placement: Option<Placement>,
  placement_source: crate::PlacementSource,
) {
  let face = match table.face_surface.get(&face_id) {
    Some(f) => f,
    None => {
      step_dbg!("step: face_surface #{face_id} not found (direct)");
      return;
    }
  };

  let surface_entity_id = entity_id_from_ph(&face.face_geometry).unwrap();

  let surface = match resolve_surface_fallback(&face.face_geometry, table) {
    Some(s) => s,
    None => {
      step_dbg!("step: face_surface #{face_id} surface resolution failed");
      return;
    }
  };

  let edge_loops = extract_edges_from_face(&face.bounds, table);
  let total_edges: usize = edge_loops.iter().map(|l| l.len()).sum();

  step_dbg!(
    "step: face_surface #{face_id} (direct) → {} loops, {} edges ({} with pcurve)",
    edge_loops.len(),
    total_edges,
    edge_loops
      .iter()
      .flat_map(|l| l.iter())
      .filter(|e| !e.pcurve_refs.is_empty())
      .count()
  );

  let is_back_face = !face.same_sense;

  faces.push(FaceSurfaceData {
    surface,
    edge_loops,
    placement,
    is_back_face,
    surface_entity_id,
    face_id,
    placement_source,
  });
}

fn extract_edges_from_face(
  bounds: &[PlaceHolder<FaceBoundHolder>],
  table: &Table,
) -> Vec<Vec<EdgeData>> {
  let mut loops: Vec<Vec<EdgeData>> = Vec::new();

  step_dbg!("step: extract_edges_from_face: {} bounds", bounds.len());

  for fb_ph in bounds {
    let fb_id = match entity_id_from_ph(fb_ph) {
      Some(id) => id,
      None => {
        step_dbg!("step:   bound entity_id not found");
        continue;
      }
    };
    let fb = match table.face_bound.get(&fb_id) {
      Some(f) => f,
      None => {
        step_dbg!("step:   face_bound #{fb_id} not found");
        continue;
      }
    };
    let loop_id = match entity_id_from_ph(&fb.bound) {
      Some(id) => id,
      None => {
        step_dbg!("step:   loop entity_id not found");
        continue;
      }
    };
    let eloop = match table.edge_loop.get(&loop_id) {
      Some(e) => e,
      None => {
        step_dbg!("step:   edge_loop #{loop_id} not found");
        continue;
      }
    };

    step_dbg!(
      "step:   loop #{loop_id} has {} edges",
      eloop.edge_list.len()
    );

    let mut loop_edges = Vec::new();
    for oe_ph in &eloop.edge_list {
      let oe_id = match entity_id_from_ph(oe_ph) {
        Some(id) => id,
        None => continue,
      };
      let oe = match table.oriented_edge.get(&oe_id) {
        Some(o) => o,
        None => {
          step_dbg!("step:     oriented_edge #{oe_id} not found");
          continue;
        }
      };
      let orientation = oe.orientation;

      let ec_id = match entity_id_from_ph(&oe.edge_element) {
        Some(id) => id,
        None => {
          step_dbg!("step:     edge_element entity_id not found");
          continue;
        }
      };
      let ec = match table.edge_curve.get(&ec_id) {
        Some(e) => e,
        None => {
          step_dbg!("step:     edge_curve #{ec_id} not found");
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

      // Extract pcurve references from the edge geometry Holder
      let pcurve_refs = extract_pcurve_refs_from_edge_curve(&ec.edge_geometry, table);

      loop_edges.push(EdgeData {
        curve_3d,
        orientation,
        same_sense: ec.same_sense,
        pcurve_refs,
      });
    }
    loops.push(loop_edges);
  }

  loops
}

/// Extract pcurve references with basis_surface entity IDs from an edge
/// curve Holder.
///
/// Navigates SurfaceCurve → associated_geometry → Pcurve, extracting each
/// 2D curve entity ID together with the pcurve's basis_surface entity ID.
/// Surface variants in PcurveOrSurface are skipped (no 2D curve to extract).
fn extract_pcurve_refs_from_edge_curve(
  curve_ph: &PlaceHolder<CurveAnyHolder>,
  table: &Table,
) -> Vec<PcurveRef> {
  let ec_id = match entity_id_from_ph(curve_ph) {
    Some(id) => id,
    None => return Vec::new(),
  };

  // Check SurfaceCurve path: SurfaceCurve → associated_geometry → Pcurve
  if let Some(sc) = table.surface_curve.get(&ec_id) {
    let mut refs = Vec::new();
    for assoc_ph in &sc.associated_geometry {
      let assoc_id = match entity_id_from_ph(assoc_ph) {
        Some(id) => id,
        None => continue,
      };
      let pcurve = match table.pcurve.get(&assoc_id) {
        Some(p) => p,
        None => continue, // skip Surface variants in PcurveOrSurface
      };
      let surface_id = entity_id_from_ph(&pcurve.basis_surface)
        .expect("pcurve.basis_surface must be a Ref at Holder level");
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
          refs.push(PcurveRef {
            curve_2d_id: item_id,
            surface_id,
          });
        }
      }
    }
    return refs;
  }

  // Check direct Pcurve path: Pcurve → reference_to_curve → entities
  if let Some(pcurve) = table.pcurve.get(&ec_id) {
    let mut refs = Vec::new();
    let surface_id = entity_id_from_ph(&pcurve.basis_surface)
      .expect("pcurve.basis_surface must be a Ref at Holder level");
    let def_rep_id = match entity_id_from_ph(&pcurve.reference_to_curve) {
      Some(id) => id,
      None => return Vec::new(),
    };
    let def_rep = match table.definitional_representation.get(&def_rep_id) {
      Some(d) => d,
      None => return Vec::new(),
    };
    for item_ph in &def_rep.representation_item {
      if let Some(item_id) = entity_id_from_ph(item_ph) {
        refs.push(PcurveRef {
          curve_2d_id: item_id,
          surface_id,
        });
      }
    }
    return refs;
  }

  Vec::new()
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
  if let Some(s) = table.surface_of_linear_extrusion.get(&id) {
    return s
      .clone()
      .into_owned(table)
      .ok()
      .map(|s| SurfaceAny::SurfaceOfLinearExtrusion(Box::new(s)));
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
  if let Some(r) = table.rational_b_spline_curve.get(&id) {
    return r
      .clone()
      .into_owned(table)
      .ok()
      .map(|c| CurveAny::RationalBSplineCurve(Box::new(c)));
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

pub fn entity_id_from_ph<T>(ph: &PlaceHolder<T>) -> Option<u64> {
  match ph {
    PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
    _ => None,
  }
}
