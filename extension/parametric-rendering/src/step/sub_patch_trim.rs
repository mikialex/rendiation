use super::*;
use crate::*;

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
  edge_loop_beziers: &[Vec<Vec<RationalBezierCurve3d<f32>>>],
  surface_entity_id: u64,
  config: &StepReadConfig,
  table: &Table,
  errors: &mut Vec<StepReadError>,
) -> Vec<TrimmedPatch> {
  // process all loops on original surface
  let global_trim3d_polylines = edge_loop_beziers
    .iter()
    .map(|edge_loop| {
      edge_loop
        .iter()
        .map(|beziers| {
          beziers
            .iter()
            .map(|bezier| {
              adaptive_tessellate_bezier_curve(bezier, config.trim_curve_tessellate_tolerance)
            })
            .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();

  let global_trim2d_polylines = global_trim3d_polylines
    .iter()
    .zip(edge_loops)
    .filter_map(|(edge_loop_polyline, edge_loop_step)| {
      let mut c_polyline = ContinuousTrimPolyline::default();
      for (continues_polylines, edge_step) in edge_loop_polyline.iter().zip(edge_loop_step) {
        let should_reverse = edge_step.same_sense != edge_step.orientation;

        // if p curve exist , use p curve for better precision
        if !edge_step.pcurve_refs.is_empty() {
          let mut pcurve_added = false;
          for pcurve_ref in &edge_step.pcurve_refs {
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

            // Normalize pcurve points into the same [0,1]² space as projections.
            let normalized = points_2d.map(|p| {
              let (nu, nv) = original_surface.normalize_pcurve_uv(p.x, p.y);
              Vec2::new(nu, nv)
            });

            let normalized = if should_reverse {
              normalized.reverse()
            } else {
              normalized
            };
            c_polyline.connect_c_polyline(normalized);
            pcurve_added = true;
          }
          assert!(
            pcurve_added,
            "edge has pcurve_refs but none matched surface {surface_entity_id}"
          );
        } else {
          let mut edge_poly = ContinuousTrimPolyline::default();
          for continues_no_edge_polylines in continues_polylines {
            // todo,we don't consider edge case that bezier curve may has shape edge
            let mut line = NoEdgeContinuousTrimPolyline::default();
            for point3d in continues_no_edge_polylines.iter() {
              // Project all 3D points to UV on the original surface.
              // UV coordinates are in [0,1]² (normalized during projection).
              if let Some((u, v, _dist)) = original_surface.project_point(
                *point3d,
                config.project_grid,
                config.project_tolerance,
                config.project_max_iter,
              ) {
                let p = Vec2::new(u, v);
                if config.validate_step_input_trim_curve_is_inbound {
                  if p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0 {
                    println!("projected point out of bounds: {p}");
                    // todo report this case
                    continue;
                  }
                }

                line.push(p);
              } else {
                println!("project failed");
                // todo report this case
              };
            }

            if !line.is_degenerate() {
              line.fix_periodic_boundary_uv_jump();
              edge_poly.push_no_edge_polyline(line);
            } else {
              println!("degenerate 2d trim line");
              // todo report this case
            }
          }

          if should_reverse {
            edge_poly = edge_poly.reverse();
          }
          c_polyline.connect_c_polyline(edge_poly);
        }
      }

      CompletedTrimPolyline::check_closed_from(c_polyline)
    })
    .collect::<Vec<_>>();

  // create trim polylines for each patch
  let mut results = Vec::new();
  for (patch_index, patch) in patches.into_iter().enumerate() {
    let mut builder = SubPatchTrimBuilder::new(patch.sub_range);
    for polyline_loop in &global_trim2d_polylines {
      builder.build_loop(polyline_loop);
    }

    let trim_loops = builder.output(config.trim_curve_reconstruct_tolerance);
    if trim_loops.is_empty() {
      let patch_center = patch.sub_range.center_point();
      let mut is_inside_loop = false;
      for p in &global_trim2d_polylines {
        if p.is_point_inside(patch_center) {
          is_inside_loop = true;
          break;
        }
      }
      if is_inside_loop {
        results.push(TrimmedPatch {
          patch,
          patch_index,
          trim_loops: Vec::new(),
        });
      }
    } else {
      results.push(TrimmedPatch {
        patch,
        patch_index,
        trim_loops,
      });
    }
  }

  results
}

struct SubPatchTrimBuilder {
  finished: Vec<CompletedTrimPolyline>,
  in_building: Vec<ContinuousTrimPolyline>,
  range: SubRange,
}

impl SubPatchTrimBuilder {
  pub fn new(range: SubRange) -> Self {
    Self {
      finished: Default::default(),
      in_building: Default::default(),
      range,
    }
  }

  pub fn output(self, error_distance_tolerance: f32) -> Vec<Vec<QuadraticBezierCurve2d<f32>>> {
    self
      .finished
      .into_iter()
      .map(|p| p.reconstruct_quadratic_bezier_curves(error_distance_tolerance))
      .collect()
  }

  fn connect_through_boundary_walk(&mut self, boundary_enter_point: Vec2<f32>) {
    let Some(current) = self.in_building.last_mut() else {
      return;
    };
    let Some(exit_point) = current.last_point() else {
      return;
    };

    if exit_point.distance_to(boundary_enter_point) < 1e-6 {
      return;
    }

    let t_exit = boundary_param(exit_point);
    let mut t_entry = boundary_param(boundary_enter_point);
    if t_entry <= t_exit {
      t_entry += 4.0;
    }

    let mut t = t_exit;
    while t < t_entry {
      let next_t = next_boundary_corner(t).min(t_entry);
      let seg_start = point_at_boundary_param(t);
      let seg_end = point_at_boundary_param(next_t);
      current.push_no_edge_polyline(NoEdgeContinuousTrimPolyline::line_segment(
        seg_start, seg_end,
      ));
      t = next_t;
    }
  }

  fn push_next_line_point(&mut self, point: Vec2<f32>) {
    if let Some(current) = self.in_building.last_mut() {
      current.push_point(point);
    } else {
      let mut poly = ContinuousTrimPolyline::default();
      poly.push_point(point);
      self.in_building.push(poly);
    }
  }

  fn build_loop(&mut self, global_trim2d_polylines: &CompletedTrimPolyline) {
    // map to sub patch uv space
    let range = self.range;
    let point_iter = global_trim2d_polylines.iter_points().map(|p| {
      let du = range.u_range.1 - range.u_range.0;
      let dv = range.v_range.1 - range.v_range.0;

      let u_local = (p.x - range.u_range.0) / du;
      let v_local = (p.y - range.v_range.0) / dv;
      Vec2::new(u_local.clamp(0.0, 1.0), v_local.clamp(0.0, 1.0))
    });

    let mut last = None;

    let mut is_previous_leaving = false;

    let mut first_open_in_point = None;
    let mut never_entered = true;

    for new in point_iter {
      if let Some(last) = &mut last {
        if let Some(next_clip) = clip_line_seg(*last, new) {
          match (next_clip.from_clipped, next_clip.to_clipped) {
            (true, true) => {
              // pass through
              if is_previous_leaving {
                self.connect_through_boundary_walk(next_clip.from);
              }
              if never_entered {
                first_open_in_point = Some(next_clip.from);
                self.push_next_line_point(next_clip.from);
              }
              self.push_next_line_point(next_clip.to);
              is_previous_leaving = true;
            }
            (true, false) => {
              // enter
              if is_previous_leaving {
                self.connect_through_boundary_walk(next_clip.from);
              }
              if never_entered {
                first_open_in_point = Some(next_clip.from);
                self.push_next_line_point(next_clip.from);
              }
              self.push_next_line_point(next_clip.to);
              is_previous_leaving = false;
            }
            (false, true) => {
              // leave
              assert!(!is_previous_leaving);
              self.push_next_line_point(next_clip.to);
              is_previous_leaving = true;
            }
            (false, false) => {
              // full inside
              assert!(!is_previous_leaving);
              self.push_next_line_point(next_clip.to);
            }
          }
          never_entered = false;
        } else {
          *last = new
        }
      } else {
        last = Some(new)
      }
    }

    if let Some(in_point) = first_open_in_point {
      self.connect_through_boundary_walk(in_point);
    }

    while let Some(poly) = self.in_building.pop() {
      if let Some(completed) = CompletedTrimPolyline::check_closed_from(poly) {
        self.finished.push(completed);
      }
    }
  }
}

fn boundary_param(p: Vec2<f32>) -> f32 {
  const EPS: f32 = 1e-5;
  if p.y.abs() < EPS {
    p.x.clamp(0.0, 1.0)
  } else if (p.x - 1.0).abs() < EPS {
    1.0 + p.y.clamp(0.0, 1.0)
  } else if (p.y - 1.0).abs() < EPS {
    3.0 - p.x.clamp(0.0, 1.0)
  } else {
    4.0 - p.y.clamp(0.0, 1.0)
  }
}

fn next_boundary_corner(t: f32) -> f32 {
  t.floor() + 1.0
}

fn point_at_boundary_param(t: f32) -> Vec2<f32> {
  let t = t % 4.0;
  if t <= 1.0 {
    Vec2::new(t, 0.0)
  } else if t <= 2.0 {
    Vec2::new(1.0, t - 1.0)
  } else if t <= 3.0 {
    Vec2::new(3.0 - t, 1.0)
  } else {
    Vec2::new(0.0, 4.0 - t)
  }
}

struct ClipResult {
  from: Vec2<f32>,
  from_clipped: bool,
  to: Vec2<f32>,
  to_clipped: bool,
}

fn clip_line_seg(from: Vec2<f32>, to: Vec2<f32>) -> Option<ClipResult> {
  let d = to - from;
  let mut t_min = 0.0;
  let mut t_max = 1.0;

  let bounds = [
    (-d.x, from.x),
    (d.x, 1.0 - from.x),
    (-d.y, from.y),
    (d.y, 1.0 - from.y),
  ];

  for (p, q) in bounds {
    if p == 0.0 {
      if q < 0.0 {
        return None;
      }
    } else {
      let t = q / p;
      if p < 0.0 {
        t_min = t_min.max(t);
      } else {
        t_max = t_max.min(t);
      }
    }
  }

  if t_min > t_max {
    return None;
  }

  Some(ClipResult {
    from: from + d * t_min,
    from_clipped: t_min > 0.0,
    to: from + d * t_max,
    to_clipped: t_max < 1.0,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fully_inside() {
    let r = clip_line_seg(Vec2::new(0.2, 0.3), Vec2::new(0.8, 0.7)).unwrap();
    assert!(!r.from_clipped);
    assert!(!r.to_clipped);
    assert!((r.from - Vec2::new(0.2, 0.3)).length() < 1e-6);
    assert!((r.to - Vec2::new(0.8, 0.7)).length() < 1e-6);
  }

  #[test]
  fn fully_outside() {
    assert!(clip_line_seg(Vec2::new(-1.0, -1.0), Vec2::new(-0.5, -0.5)).is_none());
    assert!(clip_line_seg(Vec2::new(2.0, 2.0), Vec2::new(3.0, 3.0)).is_none());
    assert!(clip_line_seg(Vec2::new(-1.0, 0.5), Vec2::new(-0.5, 0.5)).is_none());
    assert!(clip_line_seg(Vec2::new(0.5, -1.0), Vec2::new(0.5, -0.5)).is_none());
  }

  #[test]
  fn enter_from_left() {
    let r = clip_line_seg(Vec2::new(-0.5, 0.4), Vec2::new(0.6, 0.4)).unwrap();
    assert!(r.from_clipped);
    assert!(!r.to_clipped);
    assert!((r.from.x - 0.0).abs() < 1e-6);
    assert!((r.from.y - 0.4).abs() < 1e-6);
  }

  #[test]
  fn enter_from_bottom() {
    let r = clip_line_seg(Vec2::new(0.4, -0.3), Vec2::new(0.4, 0.7)).unwrap();
    assert!(r.from_clipped);
    assert!(!r.to_clipped);
    assert!((r.from.x - 0.4).abs() < 1e-6);
    assert!((r.from.y - 0.0).abs() < 1e-6);
  }

  #[test]
  fn leave_to_right() {
    let r = clip_line_seg(Vec2::new(0.3, 0.5), Vec2::new(1.5, 0.5)).unwrap();
    assert!(!r.from_clipped);
    assert!(r.to_clipped);
    assert!((r.to.x - 1.0).abs() < 1e-6);
    assert!((r.to.y - 0.5).abs() < 1e-6);
  }

  #[test]
  fn leave_to_top() {
    let r = clip_line_seg(Vec2::new(0.3, 0.2), Vec2::new(0.3, 1.5)).unwrap();
    assert!(!r.from_clipped);
    assert!(r.to_clipped);
    assert!((r.to.x - 0.3).abs() < 1e-6);
    assert!((r.to.y - 1.0).abs() < 1e-6);
  }

  #[test]
  fn pass_through() {
    let r = clip_line_seg(Vec2::new(-0.5, 0.4), Vec2::new(1.5, 0.4)).unwrap();
    assert!(r.from_clipped);
    assert!(r.to_clipped);
    assert!((r.from.x - 0.0).abs() < 1e-6);
    assert!((r.to.x - 1.0).abs() < 1e-6);
  }

  #[test]
  fn diagonal_enter_and_leave() {
    let r = clip_line_seg(Vec2::new(-0.2, -0.2), Vec2::new(1.2, 1.2)).unwrap();
    assert!(r.from_clipped);
    assert!(r.to_clipped);
    assert!((r.from - Vec2::new(0.0, 0.0)).length() < 1e-6);
    assert!((r.to - Vec2::new(1.0, 1.0)).length() < 1e-6);
  }

  #[test]
  fn degenerate_point_inside() {
    let r = clip_line_seg(Vec2::new(0.5, 0.5), Vec2::new(0.5, 0.5)).unwrap();
    assert!(!r.from_clipped);
    assert!(!r.to_clipped);
  }

  #[test]
  fn degenerate_point_outside() {
    assert!(clip_line_seg(Vec2::new(2.0, 2.0), Vec2::new(2.0, 2.0)).is_none());
  }
}
