use crate::*;

/// Validation result for a single trim-boundary curve.
#[derive(Debug)]
pub enum TrimBoundaryIssue {
  /// A point (start / ctrl / end) lies outside [0,1]².
  OutOfBounds {
    curve_idx: usize,
    point_kind: &'static str, // "start", "ctrl", "end"
    uv: (f32, f32),
  },
  /// Consecutive curve endpoints don't meet (gap > tolerance).
  Gap {
    from_curve_idx: usize,
    from_end: (f32, f32),
    to_curve_idx: usize,
    to_start: (f32, f32),
    distance: f32,
  },
}

/// Validate trim boundary curves — per-loop OOB check and within-loop
/// closure gap check.
///
/// Returns a list of issues found. An empty vec means the boundary is clean.
pub fn validate_trim_boundary(
  label: &str,
  trim_loops: &[Vec<QuadraticBezierCurve2d<f32>>],
) -> Vec<TrimBoundaryIssue> {
  let domain_eps = 5e-3;
  let gap_eps = 1e-3;
  let mut issues = Vec::new();
  let mut global_ci = 0usize;

  for (_li, loop_curves) in trim_loops.iter().enumerate() {
    let n = loop_curves.len();
    if n == 0 {
      continue;
    }

    // OOB check
    for (ci, c) in loop_curves.iter().enumerate() {
      let pts = tessellate_quadratic_bezier_2d_only(c, domain_eps * 0.5);
      for p in &pts {
        if p.x < -domain_eps
          || p.x > 1.0 + domain_eps
          || p.y < -domain_eps
          || p.y > 1.0 + domain_eps
        {
          issues.push(TrimBoundaryIssue::OutOfBounds {
            curve_idx: global_ci + ci,
            point_kind: "curve",
            uv: (p.x, p.y),
          });
          break;
        }
      }
    }

    // Within-loop closure check (circular)
    for i in 0..n {
      let j = (i + 1) % n;
      let end = (loop_curves[i].end.x, loop_curves[i].end.y);
      let start = (loop_curves[j].start.x, loop_curves[j].start.y);
      let dx = end.0 - start.0;
      let dy = end.1 - start.1;
      let dist = (dx * dx + dy * dy).sqrt();
      if dist > gap_eps {
        issues.push(TrimBoundaryIssue::Gap {
          from_curve_idx: global_ci + i,
          from_end: end,
          to_curve_idx: global_ci + j,
          to_start: start,
          distance: dist,
        });
      }
    }

    global_ci += n;
  }

  if !issues.is_empty() {
    eprintln!("[trim validation] {label}: {} issue(s)", issues.len());
    for issue in &issues {
      match issue {
        TrimBoundaryIssue::OutOfBounds {
          curve_idx,
          point_kind,
          uv,
        } => {
          eprintln!(
            "  curve[{curve_idx}].{point_kind} = ({:.4}, {:.4}) out of [0,1]²",
            uv.0, uv.1
          );
        }
        TrimBoundaryIssue::Gap {
          from_curve_idx,
          from_end,
          to_curve_idx,
          to_start,
          distance,
        } => {
          eprintln!(
            "  gap between curve[{from_curve_idx}].end=({:.4},{:.4}) and curve[{to_curve_idx}].start=({:.4},{:.4}): {:.4}",
            from_end.0, from_end.1, to_start.0, to_start.1, distance
          );
        }
      }
    }
  }

  issues
}
