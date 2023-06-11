use shadergraph::{SemanticBinding, Shader140Array, Std140};

use crate::*;

pub struct ClampedUniformList<T: Std140, const N: usize> {
  pub semantic: SemanticBinding,
  pub source: Vec<T>,
  pub gpu: Option<UniformBufferDataView<Shader140Array<T, N>>>,
}

impl<T: Std140, const N: usize> ClampedUniformList<T, N> {
  pub fn default_with(semantic: SemanticBinding) -> Self {
    Self {
      semantic,
      source: Default::default(),
      gpu: Default::default(),
    }
  }
}

impl<T: Std140 + Default, const N: usize> ClampedUniformList<T, N> {
  pub fn reset(&mut self) {
    self.source.clear();
    self.gpu.take();
  }

  pub fn update_gpu(&mut self, gpu: &GPU) -> usize {
    let mut source = vec![T::default(); N];
    for (i, light) in self.source.iter().enumerate() {
      if i >= N {
        break;
      }
      source[i] = *light;
    }
    let source = source.try_into().unwrap();
    let lights_gpu = create_uniform(source, gpu);
    self.gpu = lights_gpu.into();
    self.source.len()
  }
}

impl<T: Std140, const N: usize> ShaderPassBuilder for ClampedUniformList<T, N> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.gpu.as_ref().unwrap(), self.semantic);
  }
}

impl<T: Std140, const N: usize> ShaderHashProvider for ClampedUniformList<T, N> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.semantic.hash(hasher)
  }
}
