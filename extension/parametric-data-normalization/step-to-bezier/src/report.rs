use crate::ConvertError;
use crate::brep::FaceId;

#[derive(Default)]
pub struct ConvertReport {
  pub parse_errors: Vec<String>,
  pub brep_warnings: Vec<String>,
  pub faces: Vec<FaceReport>,
  /// Model tolerance used for this conversion (see `ConvertCtx::tol`).
  pub tol: f64,
}

pub struct FaceReport {
  pub face: FaceId,
  pub step_id: u64,
  pub surface_kind: &'static str,
  pub result: Result<FaceStats, ConvertError>,
  pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FaceStats {
  pub patch_count: usize,
  pub trim_curve_count: usize,
  pub fitted_curve_count: usize,
}
