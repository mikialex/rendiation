#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
  /// Valid STEP that this converter does not handle (yet).
  Unsupported,
  /// The file content violates the standard or is internally inconsistent.
  InvalidData,
  /// A numerical procedure failed (no convergence, degenerate geometry, tolerance exceeded).
  Numerical,
  /// Topology could not be assembled (open loops, inconsistent orientation, ...).
  Topology,
}

#[derive(Debug, Clone)]
pub struct ConvertError {
  pub kind: ErrorKind,
  /// STEP entity id the error is attached to, when known.
  pub step_id: Option<u64>,
  pub message: String,
}

pub type ConvertResult<T> = Result<T, ConvertError>;

impl ConvertError {
  pub fn new(kind: ErrorKind, step_id: Option<u64>, message: impl Into<String>) -> Self {
    Self {
      kind,
      step_id,
      message: message.into(),
    }
  }

  pub fn unsupported(step_id: Option<u64>, message: impl Into<String>) -> Self {
    Self::new(ErrorKind::Unsupported, step_id, message)
  }

  pub fn invalid(step_id: Option<u64>, message: impl Into<String>) -> Self {
    Self::new(ErrorKind::InvalidData, step_id, message)
  }

  pub fn numerical(step_id: Option<u64>, message: impl Into<String>) -> Self {
    Self::new(ErrorKind::Numerical, step_id, message)
  }

  pub fn topology(step_id: Option<u64>, message: impl Into<String>) -> Self {
    Self::new(ErrorKind::Topology, step_id, message)
  }

  /// Attach a step id if the error does not carry one yet.
  pub fn at(mut self, step_id: u64) -> Self {
    self.step_id.get_or_insert(step_id);
    self
  }
}

impl std::fmt::Display for ConvertError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.step_id {
      Some(id) => write!(f, "{:?} at #{id}: {}", self.kind, self.message),
      None => write!(f, "{:?}: {}", self.kind, self.message),
    }
  }
}

impl std::error::Error for ConvertError {}

/// Tolerances shared by all conversion stages. All values are model-space lengths except
/// `angular_tol` (radians).
#[derive(Debug, Clone, Copy)]
pub struct ConvertCtx {
  /// Model tolerance: points closer than this are the same point.
  pub tol: f64,
  /// Maximum 3D deviation allowed when a trim curve has to be approximated.
  pub fit_tol: f64,
  pub angular_tol: f64,
  /// Degree used for fitted trim curves.
  pub fit_degree: usize,
}

impl ConvertCtx {
  pub fn new(tol: f64, fit_tolerance_ratio: f64, fit_degree: usize) -> Self {
    Self {
      tol,
      fit_tol: tol * fit_tolerance_ratio,
      angular_tol: 1e-9,
      fit_degree,
    }
  }
}
