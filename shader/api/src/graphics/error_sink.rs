use crate::*;

#[derive(Clone)]
pub(crate) struct ErrorSink {
  panic_when_error: Arc<bool>,
  errors: Arc<RwLock<Vec<ShaderBuildError>>>,
}

impl ErrorSink {
  pub fn push(&self, err: ShaderBuildError) {
    if *self.panic_when_error {
      panic!("shader build error: {:#?}", err);
    }

    self.errors.write().push(err);
  }

  pub fn finish(&self) -> Vec<ShaderBuildError> {
    self.errors.write().drain(..).collect()
  }
}

impl ErrorSink {
  /// set this to true will panic when any shader build error happen. default is false.
  /// should be false in production if the shader build error should not be fatal.
  ///
  /// this is helpful in debug mode because it will locate to the first error occurred place
  pub fn new(panic_when_error: bool) -> Self {
    Self {
      panic_when_error: Arc::new(panic_when_error),
      errors: Default::default(),
    }
  }
}
