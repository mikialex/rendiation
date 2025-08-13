use crate::*;

#[derive(Clone)]
pub struct GPUSbt {
  pub inner: Arc<RwLock<Box<dyn ShaderBindingTableProvider>>>,
}

impl GPUSbt {
  pub fn new(inner: Box<dyn ShaderBindingTableProvider>) -> Self {
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }

  /// update a sbt's all hit groups at given ray_index with a rxq, assuming every blas has only one geometry.
  pub fn update(
    &self,
    changes: impl DataChanges<Key: LinearIdentified, Value = HitGroupShaderRecord>,
    ray_ty_idx: u32,
  ) {
    let mut target = self.inner.write();
    for (tlas_idx, new_hit_group) in changes.iter_update_or_insert() {
      target.config_hit_group(
        0,
        tlas_idx.alloc_index() * GLOBAL_TLAS_MAX_RAY_STRIDE,
        ray_ty_idx,
        new_hit_group,
      )
    }
  }
}
