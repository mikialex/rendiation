use std::sync::Arc;

use fast_hash_collection::FastHashSet;
use parking_lot::RwLock;

use crate::*;

/// this recorded errored model in rendering, to avoid continuous error log for errored models
#[derive(Default, Clone)]
pub struct SceneModelErrorRecorder {
  errored: Arc<RwLock<FastHashSet<EntityHandle<SceneModelEntity>>>>,
}

impl SceneModelErrorRecorder {
  pub fn report_and_filter_error<R, E: std::fmt::Debug>(
    &self,
    handle: EntityHandle<SceneModelEntity>,
    re: &Result<R, E>,
  ) {
    if let Err(e) = re {
      if self.errored.write().insert(handle) {
        log::error!(
          "unable to render scene model {handle}: {e:?}, no more errors will report for this model"
        );
      }
    }
  }
}
