use std::{cell::Cell, hash::Hash};

use rendiation_webgpu::*;

use crate::*;

#[derive(Clone)]
pub struct SceneMaterial<T> {
  pub material: T,
  pub states: MaterialStates,
}

pub trait IntoCommonSceneMaterial: Sized {
  fn into_scene_material(self) -> SceneMaterial<Self> {
    SceneMaterial {
      material: self,
      states: Default::default(),
    }
  }
}

impl<T> IntoCommonSceneMaterial for T {}

pub struct SceneMaterialGPU<T: WebGPUMaterial> {
  state_id: Cell<ValueID<MaterialStates>>,
  gpu: T::GPU,
}

impl<T: WebGPUMaterial> ShaderHashProvider for SceneMaterialGPU<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.state_id.get().hash(hasher); // todo where is updating
    self.gpu.hash_pipeline(hasher);
  }
}

impl<T> ShaderBindingProvider for SceneMaterialGPU<T>
where
  T: WebGPUMaterial,
{
  fn setup_binding(&self, builder: &mut BindingBuilder) {
    self.gpu.setup_binding(builder)
  }
}

impl<T: WebGPUMaterial> ShaderGraphProvider for SceneMaterialGPU<T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    todo!();
    self.gpu.build(builder)
  }
}

impl<T> WebGPUMaterial for SceneMaterial<T>
where
  T: Clone,
  T: WebGPUMaterial,
{
  type GPU = SceneMaterialGPU<T>;

  fn create_gpu(&self, ctx: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let gpu = self.material.create_gpu(ctx, gpu);

    let state_id = STATE_ID.lock().unwrap().get_uuid(&self.states);

    SceneMaterialGPU {
      state_id: Cell::new(state_id),
      gpu,
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.material.is_keep_mesh_shape()
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
