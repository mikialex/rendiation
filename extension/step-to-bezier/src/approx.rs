//! Optional pass converting every trim curve into polynomial quadratic Béziers.

use rendiation_algebra::*;

use crate::math::*;
use crate::*;

/// Returns a copy of `output` where every trim curve is replaced by a chain of polynomial
/// quadratic Béziers within `tolerance` measured in model space on the patch surface. Lines are
/// degree elevated exactly. Loop connectivity stays exact because fitted pieces interpolate the
/// original curve endpoints.
///
/// Curves that can not be approximated within `max_depth` subdivisions are kept unchanged and
/// counted in the returned value.
pub fn approximate_trims_quadratic(
  output: &StepConvertOutput,
  tolerance: f64,
) -> (StepConvertOutput, usize) {
  let mut failures = 0;
  let patches = output
    .patches
    .iter()
    .map(|patch| TrimmedBezierPatch {
      surface: patch.surface.clone(),
      loops: patch
        .loops
        .iter()
        .map(|trim_loop| TrimLoop {
          curves: trim_loop
            .curves
            .iter()
            .flat_map(|curve| {
              approximate_curve(&patch.surface, curve, tolerance).unwrap_or_else(|| {
                failures += 1;
                vec![curve.clone()]
              })
            })
            .collect(),
        })
        .collect(),
      flip_normal: patch.flip_normal,
      face: patch.face,
      cell: patch.cell,
    })
    .collect();

  let result = StepConvertOutput {
    patches,
    instances: output
      .instances
      .iter()
      .map(|i| FaceInstance {
        face: i.face,
        patches: i.patches.clone(),
        transform: i.transform,
        flip_normal: i.flip_normal,
      })
      .collect(),
    report: ConvertReport {
      parse_errors: output.report.parse_errors.clone(),
      brep_warnings: output.report.brep_warnings.clone(),
      faces: output
        .report
        .faces
        .iter()
        .map(|f| FaceReport {
          face: f.face,
          step_id: f.step_id,
          surface_kind: f.surface_kind,
          result: f.result.clone(),
          warnings: f.warnings.clone(),
        })
        .collect(),
      tol: output.report.tol,
    },
  };
  (result, failures)
}

const MAX_DEPTH: usize = 16;
const ERROR_SAMPLES: usize = 12;

fn approximate_curve(
  surface: &RationalBezierSurface,
  curve: &TrimCurve,
  tolerance: f64,
) -> Option<Vec<TrimCurve>> {
  let c = &curve.curve;
  if c.degree() == 1 {
    return Some(vec![TrimCurve {
      curve: c.elevate_degree(),
      ..curve.clone()
    }]);
  }
  if c.degree() == 2 && c.is_polynomial() {
    return Some(vec![curve.clone()]);
  }

  let target = |s: f64| c.eval(s);
  let error = |piece: &RationalBezierCurve2, range: (f64, f64)| {
    (0..=ERROR_SAMPLES)
      .map(|i| {
        let t = i as f64 / ERROR_SAMPLES as f64;
        let q = piece.eval(t);
        let guess = range.0 + (range.1 - range.0) * t;
        let (s, _) = closest_point_on_curve(c, q, Some(guess));
        let o = c.eval(s);
        (surface.eval(q.x, q.y) - surface.eval(o.x, o.y)).length()
      })
      .fold(0., f64::max)
  };
  let pieces = fit_bezier_2d(&target, (0., 1.), 2, &error, tolerance, MAX_DEPTH).ok()?;
  Some(
    pieces
      .into_iter()
      .map(|(_, piece)| TrimCurve {
        curve: piece,
        source: curve.source,
        exact: false,
      })
      .collect(),
  )
}
