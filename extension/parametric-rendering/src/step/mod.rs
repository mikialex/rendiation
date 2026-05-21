mod curve2d_sample;
mod curve3d_convert;
mod helpers;
mod parameter_remapping;
mod placement;
mod surface_convert;
mod surface_extent;
mod surface_project;
mod topology;

use curve2d_sample::*;
use curve3d_convert::*;
use helpers::*;
use parameter_remapping::*;
use placement::*;
use rendiation_step_reader::entities::SurfaceAny;
use rendiation_step_reader::table::Table;
use surface_convert::*;
use surface_extent::*;
use surface_project::*;
use topology::*;

use crate::*;

/// Set to `true` to enable verbose step-pipeline tracing via `println!`.
pub const STEP_DEBUG_LOG: bool = true;

/// Log a debug message when STEP_DEBUG_LOG is enabled.
/// Uses `format_args!`-style formatting call: `step_debug(format_args!(...))`.
#[allow(unused)]
pub(crate) fn step_debug(args: std::fmt::Arguments<'_>) {
  if STEP_DEBUG_LOG {
    println!("{}", args);
  }
}

/// Convenience: `step_dbg!(format_string, ...)`
macro_rules! step_dbg {
  ($($arg:tt)*) => {
    $crate::step::step_debug(format_args!($($arg)*))
  };
}
pub(crate) use step_dbg;

#[derive(Debug, Clone)]
pub struct StepReadConfig {
  pub tessellate_tolerance: f32,
  pub project_grid: usize,
  pub project_tolerance: f32,
  pub project_max_iter: usize,
  pub fit_tolerance: f32,
}

impl Default for StepReadConfig {
  fn default() -> Self {
    Self {
      tessellate_tolerance: 1e-4,
      project_grid: 8,
      project_tolerance: 1e-6,
      project_max_iter: 20,
      fit_tolerance: 1e-4,
    }
  }
}

#[derive(Debug)]
pub enum StepReadError {
  ParseError(String),
  MissingEntity { entity_type: &'static str, id: u64 },
  UnsupportedSurface { entity_type: &'static str, id: u64 },
  UnsupportedCurve { entity_type: &'static str, id: u64 },
  ConversionError(String),
}

impl std::fmt::Display for StepReadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StepReadError::ParseError(msg) => write!(f, "STEP parse error: {msg}"),
      StepReadError::MissingEntity { entity_type, id } => {
        write!(f, "Missing entity {entity_type} #{id}")
      }
      StepReadError::UnsupportedSurface { entity_type, id } => {
        write!(f, "Unsupported surface type {entity_type} (#{id})")
      }
      StepReadError::UnsupportedCurve { entity_type, id } => {
        write!(f, "Unsupported curve type {entity_type} (#{id})")
      }
      StepReadError::ConversionError(msg) => write!(f, "Conversion error: {msg}"),
    }
  }
}

impl std::error::Error for StepReadError {}

// ── Main entry ────────────────────────────────────────────────────────

pub fn read_parametric_rendering_data_from_step(
  step_str: &str,
  config: StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  let table =
    Table::from_step(step_str).ok_or_else(|| StepReadError::ParseError("Parse failed".into()))?;

  assemble_from_table(&table, &config)
}

pub fn read_parametric_rendering_data_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  assemble_from_table(table, config)
}

fn assemble_from_table(
  table: &Table,
  config: &StepReadConfig,
) -> Result<ParametricRenderingData, StepReadError> {
  let face_data_list = collect_face_surface_data(table)?;

  step_dbg!("step: {} faces collected", face_data_list.len());

  let mut trimmed_surfaces: Vec<TrimmedSurface> = Vec::new();
  let mut curves_3d: Vec<RationalBezierCurve3d<f32>> = Vec::new();

  for (fi, face_data) in face_data_list.iter().enumerate() {
    let plane_extent = if let SurfaceAny::Plane(ref p) = &face_data.surface {
      Some(compute_plane_face_extent(
        &p.position,
        &face_data.edge_loops,
        config,
      ))
    } else {
      None
    };

    // todo, check u side?
    let v_range = match &face_data.surface {
      SurfaceAny::CylindricalSurface(c) => Some(compute_axis_v_extent(
        &c.position,
        &face_data.edge_loops,
        config,
      )),
      SurfaceAny::ConicalSurface(c) => Some(compute_axis_v_extent(
        &c.position,
        &face_data.edge_loops,
        config,
      )),
      _ => None,
    };

    let (mut patches, original_surface) = match convert_and_split_any_surface_to_bezier_patches(
      &face_data.surface,
      plane_extent,
      v_range,
    ) {
      Ok(p) => p,
      Err(e) => {
        step_dbg!("step: face #{fi} surface conversion failed: {e}");
        continue;
      }
    };

    step_dbg!(
      "step: face #{fi} → {} patches, {} edges",
      patches.len(),
      face_data.edge_loops.iter().map(|l| l.len()).sum::<usize>()
    );

    let patch_trim_boundaries = process_trim_curves_for_face(
      &original_surface,
      &patches,
      &face_data.edge_loops,
      config,
      table,
    );

    if let Some(placement) = face_data.placement {
      let (origin, x_dir, _y_dir, z_dir) = placement;
      step_dbg!(
        "step: face #{fi} placement: origin=({:.3},{:.3},{:.3}) x=({:.3},{:.3},{:.3}) z=({:.3},{:.3},{:.3})",
        origin.x, origin.y, origin.z,
        x_dir.x, x_dir.y, x_dir.z,
        z_dir.x, z_dir.y, z_dir.z,
      );
      for patch in &mut patches {
        apply_placement_to_surface(&mut patch.surface, &placement);
      }
    } else {
      step_dbg!("step: face #{fi} no placement");
    }

    // Surface position debug (first two faces only)
    if fi < 2 && !patches.is_empty() {
      let p0 = patches[0].surface.evaluate(0.5, 0.5);
      step_dbg!(
        "step: face #{fi} patch center: ({:.3},{:.3},{:.3})",
        p0.x,
        p0.y,
        p0.z
      );
    }

    let n_patches = patches.len();
    for (pi, patch) in patches.iter().enumerate() {
      match &patch_trim_boundaries[pi] {
        Some(trim) => {
          // fi (face occurrence index) ensures uniqueness when the same
          // face entity appears in multiple assembly placements.
          let total_edges: usize = face_data.edge_loops.iter().map(|l| l.len()).sum();
          let debug_label = format!(
            "#{fi} FaceSurface#{}[{}/{}] {} edges={}{}",
            face_data.face_id,
            pi,
            n_patches,
            face_data.surface.surface_ty_name(),
            total_edges,
            if face_data.is_back_face {
              " (flipped)"
            } else {
              ""
            }
          );
          trimmed_surfaces.push(TrimmedSurface {
            debug_label,
            surface: patch.surface.clone(),
            trim_loops: trim.clone(),
            is_back_face: face_data.is_back_face,
          });
        }
        None => {
          step_dbg!("step: face #{fi} patch #{pi} discarded (fully trimmed)");
        }
      }
    }

    for edge in face_data.edge_loops.iter().flat_map(|l| l.iter()) {
      match convert_any_curve_to_bezier(&edge.curve_3d) {
        Ok(mut beziers) => {
          if let Some(placement) = face_data.placement {
            for curve in &mut beziers {
              apply_placement_to_curve(curve, &placement);
            }
          }
          curves_3d.extend(beziers);
        }
        Err(e) => {
          step_dbg!("step: convert_any_curve_to_bezier failed: {e:?}");
        }
      }
    }
  }

  step_dbg!(
    "step: done — {} trimmed surfaces, {} 3d curves",
    trimmed_surfaces.len(),
    curves_3d.len()
  );
  Ok(ParametricRenderingData {
    surfaces: trimmed_surfaces,
    curves_3d,
  })
}

/// Build a perimeter-bridge polyline along the [0,1]² boundary from `from`
/// to `to`, both snapped to the boundary.
///
/// Returns `None` if either point cannot be snapped.
fn build_perimeter_bridge(from: Vec2<f32>, to: Vec2<f32>, snap_eps: f32) -> Option<Vec<Vec2<f32>>> {
  let snap = |p: Vec2<f32>| -> Option<(f32, Vec2<f32>)> {
    let dists: [(f32, f32, Vec2<f32>); 4] = [
      (
        p.x.abs(),
        p.y.clamp(0.0, 1.0),
        Vec2::new(0.0, p.y.clamp(0.0, 1.0)),
      ),
      (
        (1.0 - p.y).abs(),
        1.0 + p.x.clamp(0.0, 1.0),
        Vec2::new(p.x.clamp(0.0, 1.0), 1.0),
      ),
      (
        (1.0 - p.x).abs(),
        2.0 + (1.0 - p.y.clamp(0.0, 1.0)),
        Vec2::new(1.0, p.y.clamp(0.0, 1.0)),
      ),
      (
        p.y.abs(),
        3.0 + (1.0 - p.x.clamp(0.0, 1.0)),
        Vec2::new(p.x.clamp(0.0, 1.0), 0.0),
      ),
    ];
    let mut best: Option<(f32, Vec2<f32>)> = None;
    let mut best_dist = f32::MAX;
    for (dist, t, pt) in &dists {
      if *dist < snap_eps && *dist < best_dist {
        best_dist = *dist;
        best = Some((*t, *pt));
      }
    }
    best
  };

  let (t_a, _pt_a) = snap(from)?;
  let (t_b, _pt_b) = snap(to)?;

  let dist = if t_b >= t_a {
    t_b - t_a
  } else {
    4.0 - t_a + t_b
  };
  let boundary_samples = 8;
  let n_samples = (dist * boundary_samples as f32).ceil() as usize;
  let mut pts = Vec::with_capacity(n_samples + 1);
  for s in 0..=n_samples {
    let t = if n_samples == 0 {
      t_a
    } else {
      t_a + (s as f32) * dist / (n_samples as f32)
    };
    let t = t % 4.0;
    pts.push(if t < 1.0 {
      Vec2::new(0.0, t)
    } else if t < 2.0 {
      Vec2::new(t - 1.0, 1.0)
    } else if t < 3.0 {
      Vec2::new(1.0, 1.0 - (t - 2.0))
    } else {
      Vec2::new(1.0 - (t - 3.0), 0.0)
    });
  }
  Some(pts)
}

/// Clip a UV segment `p0 → p1` against an axis-aligned rectangle
/// `[u_min, u_max] × [v_min, v_max]` (Liang-Barsky).
///
/// Returns the clipped endpoints in local `[0,1]²` coordinates, or `None`
/// if the segment lies entirely outside.
fn clip_segment_to_range(
  p0: Vec2<f32>,
  p1: Vec2<f32>,
  u_min: f32,
  u_max: f32,
  v_min: f32,
  v_max: f32,
) -> Option<(Vec2<f32>, Vec2<f32>)> {
  let dx = p1.x - p0.x;
  let dy = p1.y - p0.y;

  let mut t_entry = 0.0f32;
  let mut t_exit = 1.0f32;

  // Check 4 boundaries: left(u_min), right(u_max), bottom(v_min), top(v_max)
  let p_arr = [-dx, dx, -dy, dy];
  let q_arr = [p0.x - u_min, u_max - p0.x, p0.y - v_min, v_max - p0.y];

  for i in 0..4 {
    if p_arr[i] == 0.0 {
      if q_arr[i] < 0.0 {
        return None;
      }
    } else {
      let t = q_arr[i] / p_arr[i];
      if p_arr[i] < 0.0 {
        t_entry = t_entry.max(t);
      } else {
        t_exit = t_exit.min(t);
      }
    }
  }

  if t_entry > t_exit {
    return None;
  }

  let cx = p0.x + t_entry * dx;
  let cy = p0.y + t_entry * dy;
  let dx2 = p0.x + t_exit * dx;
  let dy2 = p0.y + t_exit * dy;

  let du = u_max - u_min;
  let dv = v_max - v_min;
  if du <= 0.0 || dv <= 0.0 {
    return None;
  }

  Some((
    Vec2::new((cx - u_min) / du, (cy - v_min) / dv),
    Vec2::new((dx2 - u_min) / du, (dy2 - v_min) / dv),
  ))
}

/// Clip a contiguous projected polyline segment-by-segment against each
/// patch's SubRange, appending clipped sub-polylines.
fn flush_projected_to_patches(
  ei: usize,
  projected: &mut Vec<Vec2<f32>>,
  patches: &[SurfaceSubPatch],
  per_patch: &mut [Vec<Vec<Vec2<f32>>>],
  global_polylines: &mut [Vec<Vec2<f32>>],
) {
  if projected.len() < 2 {
    projected.clear();
    return;
  }

  for (pi, patch) in patches.iter().enumerate() {
    let r = &patch.sub_range;
    let mut current: Vec<Vec2<f32>> = Vec::new();

    for w in projected.windows(2) {
      let (p0, p1) = (w[0], w[1]);
      if let Some((c0, c1)) =
        clip_segment_to_range(p0, p1, r.u_range.0, r.u_range.1, r.v_range.0, r.v_range.1)
      {
        if current.is_empty() {
          current.push(c0);
        } else {
          let last = *current.last().unwrap();
          if (last - c0).length() > 1e-6 {
            // Disconnect — start new sub-polyline
            if current.len() >= 2 {
              per_patch[pi][ei].extend(current.drain(..));
            }
            current.clear();
            current.push(c0);
          }
        }
        current.push(c1);
      } else {
        // Segment outside patch — flush current
        if current.len() >= 2 {
          per_patch[pi][ei].extend(current.drain(..));
        }
        current.clear();
      }
    }
    if current.len() >= 2 {
      per_patch[pi][ei].extend(current);
    }
  }

  global_polylines[ei].extend(projected.drain(..));
}

/// Project 3D tessellation points onto the original surface once, then
/// distribute the resulting (u,v) coordinates to the correct Bezier patch
/// by checking each patch's parameter range.
///
/// Returns `Some(trim_boundary)` per patch to keep, or `None` for patches
/// that are fully outside the trimmed region and should be discarded.
fn process_trim_curves_for_face(
  original_surface: &OriginalSurface,
  patches: &[SurfaceSubPatch],
  edge_loops: &[Vec<EdgeData>],
  config: &StepReadConfig,
  table: &Table,
) -> Vec<Option<Vec<Vec<QuadraticBezierCurve2d<f32>>>>> {
  let n_patches = patches.len();
  let total_edges: usize = edge_loops.iter().map(|l| l.len()).sum();
  if n_patches == 0 || total_edges == 0 {
    return std::iter::repeat_with(|| Some(vec![]))
      .take(n_patches)
      .collect();
  }

  // Flatten edges with loop-index tracking for later per-loop reconstruction.
  let mut edges: Vec<&EdgeData> = Vec::with_capacity(total_edges);
  let mut edge_loop_idx: Vec<usize> = Vec::with_capacity(total_edges);
  for (li, loop_edges) in edge_loops.iter().enumerate() {
    for edge in loop_edges {
      edges.push(edge);
      edge_loop_idx.push(li);
    }
  }

  let n_edges = total_edges;
  let mut per_patch_polylines: Vec<Vec<Vec<Vec2<f32>>>> =
    vec![vec![Vec::new(); n_edges]; n_patches];

  let mut global_polylines: Vec<Vec<Vec2<f32>>> = vec![Vec::new(); n_edges];

  let proj_dist_tolerance = 1e-2;

  for (ei, edge) in edges.iter().enumerate() {
    if try_pcurve_for_all_patches(
      ei,
      edge,
      patches,
      table,
      &mut per_patch_polylines,
      &mut global_polylines,
    ) {
      continue;
    }

    let beziers = match convert_any_curve_to_bezier(&edge.curve_3d) {
      Ok(b) => b,
      Err(_) => continue,
    };

    let mut all_3d_points: Vec<Vec3<f32>> = Vec::new();
    for curve in &beziers {
      let pts = adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance);
      all_3d_points.extend(pts);
    }

    if all_3d_points.is_empty() {
      continue;
    }

    // Project all 3D points to UV on the original surface.
    let mut projected: Vec<Vec2<f32>> = Vec::with_capacity(all_3d_points.len());
    let mut prev_u: Option<f32> = None;
    for &p3 in &all_3d_points {
      let Some((u, v, dist)) = original_surface.project_point(
        p3,
        config.project_grid,
        config.project_tolerance,
        config.project_max_iter,
      ) else {
        continue;
      };
      if dist > proj_dist_tolerance {
        continue;
      }
      // todo, check v?
      // Detect angular wrap-around (only for surfaces with periodic U:
      // cylinder, cone, sphere, torus — Plane/NURBS/extrusion have no period).
      if let Some(prev) = prev_u {
        if original_surface.is_periodic() && (u - prev).abs() > std::f32::consts::PI {
          flush_projected_to_patches(
            ei,
            &mut projected,
            patches,
            &mut per_patch_polylines,
            &mut global_polylines,
          );
        }
      }
      prev_u = Some(u);
      projected.push(Vec2::new(u, v));
    }
    flush_projected_to_patches(
      ei,
      &mut projected,
      patches,
      &mut per_patch_polylines,
      &mut global_polylines,
    );
  }

  // Reverse polylines for edges traversed backwards.
  for (ei, edge) in edges.iter().enumerate() {
    if edge.same_sense != edge.orientation {
      for pi in 0..n_patches {
        per_patch_polylines[pi][ei].reverse();
      }
    }
  }

  // Phase 2: classify empty patches, then reconstruct per-loop trim boundaries.
  let boundary_epsilon = config.fit_tolerance * 10.0;

  let keep_mask = classify_empty_patches(patches, global_polylines, &per_patch_polylines);

  per_patch_polylines
    .into_iter()
    .enumerate()
    .map(|(pi, edge_polys)| {
      if !keep_mask[pi] {
        return None;
      }
      if edge_polys.iter().all(|p| p.is_empty()) {
        return Some(Vec::new());
      }

      let snap_eps = (boundary_epsilon * 5.0).max(5e-3);
      let mut loop_quadratics: Vec<Vec<QuadraticBezierCurve2d<f32>>> = Vec::new();

      // Process each loop independently — each loop produces its own vec.
      for li in 0..edge_loops.len() {
        let loop_edge_indices: Vec<usize> = edge_loop_idx
          .iter()
          .enumerate()
          .filter(|(_, &l)| l == li)
          .map(|(ei, _)| ei)
          .collect();

        if loop_edge_indices.is_empty() {
          continue;
        }

        // Collect edges that actually have polylines in this patch.
        let active: Vec<(usize, &Vec<Vec2<f32>>)> = loop_edge_indices
          .iter()
          .filter_map(|&ei| {
            if edge_polys[ei].len() >= 2 {
              Some((ei, &edge_polys[ei]))
            } else {
              None
            }
          })
          .collect();
        if active.is_empty() {
          continue;
        }

        let mut qs = Vec::new();
        let n_active = active.len();
        for k in 0..n_active {
          let (_ei, poly) = active[k];
          qs.extend(crate::surface_trim::reconstruct_boundary(
            poly,
            config.fit_tolerance,
          ));

          // Bridge to the next active edge if endpoints don't meet.
          let (_, next_poly) = active[(k + 1) % n_active];
          let this_end = poly.last().copied();
          let next_start = next_poly.first().copied();
          if let (Some(end), Some(start)) = (this_end, next_start) {
            let gap = (end - start).length();
            if gap > boundary_epsilon * 0.5 {
              if let Some(bridge) = build_perimeter_bridge(end, start, snap_eps) {
                if bridge.len() >= 2 {
                  qs.extend(crate::surface_trim::reconstruct_boundary(
                    &bridge,
                    config.fit_tolerance,
                  ));
                }
              }
            }
          }
        }

        if !qs.is_empty() {
          loop_quadratics.push(qs);
        }
      }

      Some(loop_quadratics)
    })
    .collect()
}

/// Build a global trim boundary from non-empty patches and classify empty
/// patches as inside (keep) or outside (discard) the trimmed region.
///
/// Returns a bool per patch: `true` = keep, `false` = discard.
fn classify_empty_patches(
  patches: &[SurfaceSubPatch],
  global_polylines: Vec<Vec<Vec2<f32>>>,
  per_patch_polylines: &[Vec<Vec<Vec2<f32>>>],
) -> Vec<bool> {
  // For each empty patch, test if its center is inside the global boundary
  let mut keep = vec![true; patches.len()];

  for (pi, edge_polys) in per_patch_polylines.iter().enumerate() {
    if edge_polys.iter().any(|p| !p.is_empty()) {
      continue; // non-empty patch → already classified as keep
    }

    // Map patch center to normalized global space
    let cu = (patches[pi].sub_range.u_range.0 + patches[pi].sub_range.u_range.1) * 0.5;
    let cv = (patches[pi].sub_range.v_range.0 + patches[pi].sub_range.v_range.1) * 0.5;
    let test_pt = Vec2::new(cu, cv);

    // Point-in-polygon test against each closed component.
    // If inside any component, keep the patch; otherwise discard.
    let inside = global_polylines
      .iter()
      .any(|comp| point_in_polygon(test_pt, comp));
    keep[pi] = inside;
  }

  keep
}

/// Try pcurve path for all patches at once (remaps pcurve points to each patch).
/// Returns true if the pcurve path was used successfully.
fn try_pcurve_for_all_patches(
  ei: usize,
  edge: &EdgeData,
  patches: &[SurfaceSubPatch],
  table: &Table,
  per_patch_polylines: &mut [Vec<Vec<Vec2<f32>>>],
  global_polylines: &mut [Vec<Vec2<f32>>],
) -> bool {
  if edge.pcurve_entity_ids.is_empty() {
    return false;
  }

  for &entity_id in &edge.pcurve_entity_ids {
    let points_2d = match extract_2d_curve_points(table, entity_id) {
      Ok(p) => p,
      Err(_) => continue,
    };
    if points_2d.is_empty() {
      continue;
    }

    let mut any_mapped = false;
    for (pi, patch) in patches.iter().enumerate() {
      global_polylines[ei] = points_2d.clone();
      let remapped = remap_2d_points_to_patch(&points_2d, &patch.sub_range);
      if !remapped.is_empty() {
        per_patch_polylines[pi][ei] = remapped;
        any_mapped = true;
      }
    }
    if any_mapped {
      return true;
    }
  }
  false
}
