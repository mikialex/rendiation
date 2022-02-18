use shadergraph::*;

pub use crate::*;

impl<T: ShaderGraphNodeType> ShaderUniformProvider for UniformBuffer<T> {
  type Node = T;
}

impl<T: ShaderGraphNodeType> ShaderUniformProvider for UniformBufferData<T> {
  type Node = T;
}

pub struct SemanticUniformCell<T, S> {
  pub s: S,
  pub res: T,
}

pub type SemanticGPUTexture2d<T> = SemanticUniformCell<T, GPUTexture2d>;

impl<T: 'static> ShaderUniformProvider for SemanticGPUTexture2d<T> {
  type Node = ShaderTexture;
}

pub type SemanticGPUSampler<T> = SemanticUniformCell<T, GPUSampler>;

impl<T: 'static> ShaderUniformProvider for SemanticGPUSampler<T> {
  type Node = ShaderSampler;
}
