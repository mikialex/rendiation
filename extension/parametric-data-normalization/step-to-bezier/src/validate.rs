//! Validation metrics for a conversion result.

use fast_hash_collection::FastHashMap;
use rendiation_algebra::*;

use crate::brep::EdgeId;
use crate::curve::{Edge3, build_edge3};
use crate::math::*;
use crate::*;

#[derive(Debug, Default)]
pub struct ValidationReport {
  pub patch_count: usize,
  pub trim_curve_count: usize,
  /// Loops whose consecutive curve endpoints do not match exactly.
  pub open_loops: usize,
  /// Trim curves whose control polygon leaves `[0,1]²` by more than `DOMAIN_EPS`.
  pub out_of_domain_curves: usize,
  /// Patches whose loops enclose a non positive total signed area (wrong orientation or
  /// degenerate trimming).
  pub non_positive_area_patches: usize,
  /// Max distance between edge sourced trim curves mapped through the patch and the original 3D
  /// edge, over all samples.
  pub max_edge_deviation: f64,
  /// Number of trim curves whose deviation exceeds `session.ctx.tol`.
  pub edge_deviation_violations: usize,
  pub messages: Vec<String>,
}

const DOMAIN_EPS: f64 = 1e-9;
const EDGE_SAMPLES: usize = 16;
const MAX_MESSAGES: usize = 50;

pub fn validate(session: &ConvertSession) -> ValidationReport {
  let mut report = ValidationReport {
    patch_count: session.model.patches.len(),
    ..Default::default()
  };
  let mut edges: FastHashMap<EdgeId, Option<Edge3>> = FastHashMap::default();

  for (patch_index, patch) in session.model.patches.iter().enumerate() {
    let mut total_area = 0.;
    for (loop_index, trim_loop) in patch.loops.iter().enumerate() {
      report.trim_curve_count += trim_loop.curves.len();
      if !is_closed_exactly(trim_loop) {
        report.open_loops += 1;
        push_message(
          &mut report,
          format!("patch {patch_index} loop {loop_index}: not closed exactly"),
        );
      }
      total_area += trim_loop
        .curves
        .iter()
        .map(|c| signed_area_contribution(&c.curve))
        .sum::<f64>();

      for curve in &trim_loop.curves {
        let (min, max) = curve.curve.control_bbox();
        if min.x < -DOMAIN_EPS
          || min.y < -DOMAIN_EPS
          || max.x > 1. + DOMAIN_EPS
          || max.y > 1. + DOMAIN_EPS
        {
          report.out_of_domain_curves += 1;
        }

        let TrimCurveSource::Edge { edge, .. } = curve.source else {
          continue;
        };
        let edge3 = edges
          .entry(edge)
          .or_insert_with(|| build_edge3(&session.brep, edge, &session.ctx).ok());
        let Some(edge3) = edge3 else {
          continue;
        };
        let deviation = edge_deviation(&patch.surface, &curve.curve, edge3);
        report.max_edge_deviation = report.max_edge_deviation.max(deviation);
        if deviation > session.ctx.tol {
          report.edge_deviation_violations += 1;
          push_message(
            &mut report,
            format!(
              "patch {patch_index} (face {:?}): trim curve of edge {:?} deviates {deviation:e} from the 3D edge",
              patch.face, edge
            ),
          );
        }
      }
    }
    if total_area <= 0. {
      report.non_positive_area_patches += 1;
      push_message(
        &mut report,
        format!("patch {patch_index}: total trimmed area {total_area:e} is not positive"),
      );
    }
  }
  report
}

fn push_message(report: &mut ValidationReport, message: String) {
  if report.messages.len() < MAX_MESSAGES {
    report.messages.push(message);
  }
}

fn is_closed_exactly(trim_loop: &TrimLoop) -> bool {
  let curves = &trim_loop.curves;
  !curves.is_empty()
    && (0..curves.len()).all(|i| {
      let next = &curves[(i + 1) % curves.len()];
      curves[i].curve.end() == next.curve.start()
    })
}

/// `∮ x dy` over one curve by Gauss-Legendre quadrature (summed over a loop it is the enclosed
/// signed area).
fn signed_area_contribution(curve: &RationalBezierCurve2) -> f64 {
  const NODES: [(f64, f64); 5] = [
    (0.0, 0.568_888_888_888_888_9),
    (-0.538_469_310_105_683, 0.478_628_670_499_366_5),
    (0.538_469_310_105_683, 0.478_628_670_499_366_5),
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
  ];
  const INTERVALS: usize = 8;
  let mut sum = 0.;
  for k in 0..INTERVALS {
    let a = k as f64 / INTERVALS as f64;
    let half = 0.5 / INTERVALS as f64;
    for (x, w) in NODES {
      let t = a + half * (x + 1.);
      let (p, d) = curve.eval_with_derivative(t);
      sum += w * half * p.x * d.y;
    }
  }
  sum
}

/// Max over samples of the distance from `surface(curve(t))` to the 3D edge.
fn edge_deviation(
  surface: &RationalBezierSurface,
  curve: &RationalBezierCurve2,
  edge3: &Edge3,
) -> f64 {
  (0..=EDGE_SAMPLES)
    .map(|i| {
      let uv = curve.eval(i as f64 / EDGE_SAMPLES as f64);
      let p = surface.eval(uv.x, uv.y);
      closest_point_on_chain(&edge3.segments, p).map_or(0., |(_, _, d)| d)
    })
    .fold(0., f64::max)
}
