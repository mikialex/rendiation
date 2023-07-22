use shadergraph::{Shader140Array, ShaderStructMemberValueNodeType, Std140};

use crate::*;

pub struct ClampedUniformList<T: Std140, const N: usize> {
  pub source: Vec<T>,
  pub gpu: Option<UniformBufferDataView<Shader140Array<T, N>>>,
}

impl<T: Std140, const N: usize> Default for ClampedUniformList<T, N> {
  fn default() -> Self {
    Self {
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

  pub fn update_gpu(&mut self, gpu: &GPUDevice) -> usize {
    let mut source = vec![T::default(); N];
    for (i, light) in self.source.iter().enumerate() {
      if i >= N {
        break;
      }
      source[i] = *light;
    }
    let source = source.try_into().unwrap();
    let lights_gpu = create_uniform2(source, gpu);
    self.gpu = lights_gpu.into();
    self.source.len()
  }
}

impl<T, const N: usize> ShaderPassBuilder for ClampedUniformList<T, N>
where
  T: Std140 + ShaderStructMemberValueNodeType,
{
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.gpu.as_ref().unwrap());
  }
}

impl<T: Std140, const N: usize> ShaderHashProvider for ClampedUniformList<T, N> {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
}
