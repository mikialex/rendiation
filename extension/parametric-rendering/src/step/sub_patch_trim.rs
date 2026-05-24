use super::*;
use crate::*;

// Semantic types for trim polyline data

/// A single continuous polyline in UV space.
pub type TrimPolyline = Vec<Vec2<f32>>;

/// Trim segments for one edge within one patch.
/// May contain multiple polylines (split at clip discontinuities or patch crossings).
#[derive(Clone, Default)]
struct EdgeTrimSegments(Vec<TrimPolyline>);

impl EdgeTrimSegments {
  fn push(&mut self, poly: TrimPolyline) {
    self.0.push(poly);
  }

  fn is_empty(&self) -> bool {
    self.0.iter().all(|p| p.is_empty())
  }

  fn iter(&self) -> impl Iterator<Item = &TrimPolyline> {
    self.0.iter()
  }

  fn iter_mut(&mut self) -> impl Iterator<Item = &mut TrimPolyline> {
    self.0.iter_mut()
  }
}

/// All trim polyline data, indexed as [patch_index][edge_index] → EdgeTrimSegments.
#[derive(Clone)]
struct PatchTrimTable(Vec<Vec<EdgeTrimSegments>>);

impl PatchTrimTable {
  fn new(n_patches: usize, n_edges: usize) -> Self {
    Self(vec![vec![EdgeTrimSegments::default(); n_edges]; n_patches])
  }

  fn edge_mut(&mut self, pi: usize, ei: usize) -> &mut EdgeTrimSegments {
    &mut self.0[pi][ei]
  }
}

/// Global trim boundary polylines — one per edge, in surface parameter space.
type GlobalTrimPolylines = Vec<TrimPolyline>;

/// A surface sub-patch together with its reconstructed trim loops.
pub struct TrimmedPatch {
  pub patch: SurfaceSubPatch,
  pub patch_index: usize,
  pub trim_loops: Vec<Vec<QuadraticBezierCurve2d<f32>>>,
}

// Main entry

/// Project 3D tessellation points onto the original surface once, then
/// distribute the resulting (u,v) coordinates to the correct Bezier patch
/// by checking each patch's parameter range.
///
/// Takes ownership of `patches` and returns kept patches with their
/// reconstructed trim loops. Discarded patches are logged and filtered out.
pub fn process_trim_curves_for_face(
  original_surface: &OriginalSurface,
  patches: Vec<SurfaceSubPatch>,
  edge_loops: &[Vec<EdgeData>],
  surface_entity_id: u64,
  config: &StepReadConfig,
  table: &Table,
  edge_beziers: &[Vec<RationalBezierCurve3d<f32>>],
  errors: &mut Vec<StepReadError>,
) -> Vec<TrimmedPatch> {
  let n_patches = patches.len();
  let total_edges: usize = edge_loops.iter().map(|l| l.len()).sum();
  if n_patches == 0 || total_edges == 0 {
    return Vec::new();
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
  let mut per_patch = PatchTrimTable::new(n_patches, n_edges);
  let mut global_polylines: GlobalTrimPolylines = vec![TrimPolyline::new(); n_edges];

  let proj_dist_tolerance = 1e-2;

  for (ei, edge) in edges.iter().enumerate() {
    if try_pcurve_for_all_patches(
      ei,
      edge,
      &patches,
      table,
      original_surface,
      surface_entity_id,
      &mut per_patch,
      &mut global_polylines,
      errors,
    ) {
      continue;
    }

    let all_3d_points: Vec<_> = edge_beziers[ei]
      .iter()
      .flat_map(|curve| adaptive_tessellate_bezier_curve(curve, config.tessellate_tolerance))
      .collect();

    // Project all 3D points to UV on the original surface.
    // UV coordinates are in [0,1]² (normalized during projection).
    let projected: TrimPolyline = all_3d_points
      .into_iter()
      .filter_map(|p3| {
        let Some((u, v, dist)) = original_surface.project_point(
          p3,
          config.project_grid,
          config.project_tolerance,
          config.project_max_iter,
        ) else {
          // todo report this case
          return None;
        };
        if dist > proj_dist_tolerance {
          // todo report this case
          return None;
        }
        Some(Vec2::new(u, v))
      })
      .collect();

    if config.validate_step_input_trim_curve_is_inbound {
      for p in &projected {
        if p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0 {
          println!("projected point out of bounds: {p}");
          break;
        }
      }
    }

    flush_projected_to_patches(
      ei,
      projected,
      &patches,
      &mut per_patch,
      &mut global_polylines,
    );
  }

  // Reverse polylines for edges traversed backwards.
  for (ei, edge) in edges.iter().enumerate() {
    if edge.same_sense != edge.orientation {
      for pi in 0..n_patches {
        for seg in per_patch.edge_mut(pi, ei).iter_mut() {
          seg.reverse();
        }
      }
    }
  }

  // Phase 2: classify empty patches, then reconstruct per-loop trim boundaries.
  let boundary_epsilon = config.fit_tolerance * 10.0;

  // Build per-loop closed polygons once, for testing empty patches.
  let loop_polygons = build_loop_polygons(&global_polylines, &edge_loop_idx, edge_loops.len());

  per_patch
    .0
    .into_iter()
    .enumerate()
    .filter_map(|(pi, edge_polys)| {
      if edge_polys.iter().all(|e| e.is_empty()) {
        // Empty patch: test center against trim boundary.
        if !is_patch_inside_trim(&patches[pi], &loop_polygons) {
          step_dbg!("step: patch #{pi} discarded (fully trimmed)");
          return None;
        }
        return Some(TrimmedPatch {
          patch: patches[pi].clone(),
          patch_index: pi,
          trim_loops: Vec::new(),
        });
      }

      let snap_eps = (boundary_epsilon * 5.0).max(5e-3);
      let mut trim_loops: Vec<Vec<QuadraticBezierCurve2d<f32>>> = Vec::new();

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

        // Flatten all segments from all edges in loop order.
        let mut active: Vec<(usize, &TrimPolyline)> = Vec::new();
        for &ei in &loop_edge_indices {
          for seg in edge_polys[ei].iter() {
            if seg.len() >= 2 {
              active.push((ei, seg));
            }
          }
        }
        if active.is_empty() {
          continue;
        }

        let mut qs = Vec::new();
        let n_active = active.len();
        for k in 0..n_active {
          let (_ei, poly) = active[k];
          qs.extend(reconstruct_boundary(poly, config.fit_tolerance));

          // Bridge to the next active segment if endpoints don't meet.
          let (_, next_poly) = active[(k + 1) % n_active];
          let this_end = poly.last().copied();
          let next_start = next_poly.first().copied();
          if let (Some(end), Some(start)) = (this_end, next_start) {
            let gap = (end - start).length();
            if gap > boundary_epsilon * 0.5 {
              if let Some(bridge) = build_perimeter_bridge(end, start, snap_eps) {
                if bridge.len() >= 2 {
                  qs.extend(reconstruct_boundary(&bridge, config.fit_tolerance));
                }
              }
            }
          }
        }

        if !qs.is_empty() {
          trim_loops.push(qs);
        }
      }

      Some(TrimmedPatch {
        patch: patches[pi].clone(),
        patch_index: pi,
        trim_loops,
      })
    })
    .collect()
}

// Pcurve path

/// Try pcurve path for all patches at once (remaps pcurve points to each patch).
/// Returns true if the pcurve path was used successfully.
///
/// Filters pcurves to only use those whose basis_surface matches the face's
/// surface (via `surface_entity_id`). Pcurves defined on other surfaces are
/// skipped — they belong to adjacent faces sharing the same edge.
fn try_pcurve_for_all_patches(
  ei: usize,
  edge: &EdgeData,
  patches: &[SurfaceSubPatch],
  table: &Table,
  original_surface: &OriginalSurface,
  surface_entity_id: u64,
  per_patch: &mut PatchTrimTable,
  global_polylines: &mut GlobalTrimPolylines,
  errors: &mut Vec<StepReadError>,
) -> bool {
  if edge.pcurve_refs.is_empty() {
    return false;
  }

  for pcurve_ref in &edge.pcurve_refs {
    // Skip pcurves that belong to a different face's surface.
    if surface_entity_id != pcurve_ref.surface_id {
      continue;
    }

    let points_2d = match extract_2d_curve_points(table, pcurve_ref.curve_2d_id) {
      Ok(p) => p,
      Err(e) => {
        errors.push(e);
        continue;
      }
    };
    if points_2d.len() < 2 {
      errors.push(StepReadError::ConversionError(format!(
        "pcurve entity #{} has fewer than 2 points (got {})",
        pcurve_ref.curve_2d_id,
        points_2d.len()
      )));
      continue;
    }

    // Normalize pcurve points into the same [0,1]² space as projections.
    let normalized: TrimPolyline = points_2d
      .iter()
      .map(|p| {
        let (nu, nv) = original_surface.normalize_pcurve_uv(p.x, p.y);
        Vec2::new(nu, nv)
      })
      .collect();

    let mut any_mapped = false;
    for (pi, patch) in patches.iter().enumerate() {
      global_polylines[ei] = normalized.clone();
      let remapped = remap_2d_points_to_patch(&normalized, &patch.sub_range);
      if !remapped.is_empty() {
        per_patch.edge_mut(pi, ei).0 = vec![remapped];
        any_mapped = true;
      }
    }
    if any_mapped {
      return true;
    }
  }
  false
}

// Projection flushing

/// Clip a contiguous projected polyline segment-by-segment against each
/// patch's SubRange, appending clipped sub-polylines.
fn flush_projected_to_patches(
  ei: usize,
  projected: TrimPolyline,
  patches: &[SurfaceSubPatch],
  per_patch: &mut PatchTrimTable,
  global_polylines: &mut GlobalTrimPolylines,
) {
  if projected.len() < 2 {
    return;
  }

  for (pi, patch) in patches.iter().enumerate() {
    let r = &patch.sub_range;
    let mut current: TrimPolyline = Vec::new();

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
            if current.len() >= 2 {
              per_patch
                .edge_mut(pi, ei)
                .push(std::mem::take(&mut current));
            }
            current.clear();
            current.push(c0);
          }
        }
        current.push(c1);
      } else {
        if current.len() >= 2 {
          per_patch
            .edge_mut(pi, ei)
            .push(std::mem::take(&mut current));
        }
        current.clear();
      }
    }
    if current.len() >= 2 {
      per_patch.edge_mut(pi, ei).push(current);
    }
  }

  global_polylines[ei].extend(projected);
}

// Empty patch classification

/// Concatenate per-edge polylines into per-loop closed polygons.
fn build_loop_polygons(
  global_polylines: &GlobalTrimPolylines,
  edge_loop_idx: &[usize],
  n_loops: usize,
) -> Vec<TrimPolyline> {
  let mut loop_polygons: Vec<TrimPolyline> = vec![TrimPolyline::new(); n_loops];
  for (ei, poly) in global_polylines.iter().enumerate() {
    if poly.len() >= 2 {
      let li = edge_loop_idx[ei];
      loop_polygons[li].extend(poly.iter().copied());
    }
  }
  loop_polygons
}

/// Test whether an empty patch is inside the trim boundary.
fn is_patch_inside_trim(patch: &SurfaceSubPatch, loop_polygons: &[TrimPolyline]) -> bool {
  let cu = (patch.sub_range.u_range.0 + patch.sub_range.u_range.1) * 0.5;
  let cv = (patch.sub_range.v_range.0 + patch.sub_range.v_range.1) * 0.5;
  let test_pt = Vec2::new(cu, cv);
  loop_polygons
    .iter()
    .any(|poly| poly.len() >= 3 && point_in_polygon(test_pt, poly))
}

// Perimeter bridge

/// Build a perimeter-bridge polyline along the [0,1]² boundary from `from`
/// to `to`, both snapped to the boundary.
///
/// Returns `None` if either point cannot be snapped.
fn build_perimeter_bridge(from: Vec2<f32>, to: Vec2<f32>, snap_eps: f32) -> Option<TrimPolyline> {
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

// Liang-Barsky UV clipping

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
