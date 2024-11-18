use crate::*;

pub struct ReactiveQuerySbtMaintainer {
  updater: MultiUpdateContainer<Box<dyn ShaderBindingTableProvider>>,
}

impl ReactiveQuerySbtMaintainer {
  pub fn with_identical_source(self) -> Self {
    // todo
    self
  }

  pub fn with_given_ray_ty_sbt_update(
    mut self,
    ray_ty_idx: u32,
    source: impl ReactiveQuery<Key = u32, Value = HitGroupShaderRecord>,
  ) -> Self {
    self
      .updater
      .add_source(ReactiveQuerySbtUpdater { ray_ty_idx, source });
    self
  }
}

/// update a sbt's all hit groups at given ray_index with a rxq, assuming every blas has only one geometry.
pub struct ReactiveQuerySbtUpdater<T> {
  pub ray_ty_idx: u32,
  pub source: T,
}

impl<T> QueryBasedUpdate<Box<dyn ShaderBindingTableProvider>> for ReactiveQuerySbtUpdater<T>
where
  T: ReactiveQuery<Key = u32, Value = HitGroupShaderRecord>,
{
  fn update_target(&mut self, target: &mut Box<dyn ShaderBindingTableProvider>, cx: &mut Context) {
    let (change, _) = self.source.poll_changes(cx);

    for (tlas_idx, change) in change.iter_key_value() {
      match change {
        ValueChange::Delta(new_hit_group, _) => {
          target.config_hit_group(tlas_idx, self.ray_ty_idx, new_hit_group)
        }
        ValueChange::Remove(_) => {}
      }
    }
  }
}
