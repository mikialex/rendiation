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
        let should_reverse = edge_step.same_sense != edge_step.orientation; // todo

        // if p curve exist , use p curve for better precision
        if !edge_step.pcurve_refs.is_empty() {
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

            c_polyline.connect_c_polyline(normalized);
          }
        } else {
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
                    break;
                  }
                }

                line.push(p);
              } else {
                // todo report this case
              };
            }

            if !line.is_degenerate() {
              c_polyline.push_no_edge_polyline(line);
            }
          }
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
    // assert boundary_enter_point is at boundary
    // assert last point is at boundary
    // if they are same point, just skip push
    todo!()
  }

  fn push_next_line_point(&mut self, point: Vec2<f32>) {
    //
    todo!()
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
          assert!(!is_previous_leaving);
          *last = new
        }
      } else {
        last = Some(new)
      }
    }

    if let Some(in_point) = first_open_in_point {
      assert!(is_previous_leaving);
      self.connect_through_boundary_walk(in_point);
    }
  }
}

struct ClipResult {
  from: Vec2<f32>,
  from_clipped: bool,
  to: Vec2<f32>,
  to_clipped: bool,
}

// Liang-Barsky clipping
fn clip_line_seg(from: Vec2<f32>, to: Vec2<f32>) -> Option<ClipResult> {
  todo!()
}
