use crate::*;

pub struct SceneRayTracingAOFeature {
  desc: GPURaytracingPipelineDescriptor,
  // should we keep this?
  pipeline: Box<dyn GPURaytracingPipelineProvider>,
  sbt: Box<dyn ShaderBindingTableProvider>,
}

impl SceneRayTracingAOFeature {
  pub fn new(gpu: &GPU, tlas_size: Box<dyn Stream<Item = u32>>) -> Self {
    todo!()
  }

  pub fn render(&self, input: GPU2DTextureView) -> GPU2DTextureView {
    todo!()
  }
}

struct ReactiveQuerySbtMaintainer {
  updater: MultiUpdateContainer<Box<dyn ShaderBindingTableProvider>>,
}

impl ReactiveQuerySbtMaintainer {
  // pub fn with_identical_source() -> Self{
  // }
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
