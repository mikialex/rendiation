use shadergraph::*;

pub use crate::*;

impl<T: ShaderGraphNodeType> ShaderUniformProvider for UniformBufferView<T> {
  type Node = T;
}

impl<T: ShaderGraphNodeType> ShaderUniformProvider for UniformBufferDataView<T> {
  type Node = T;
}

impl ShaderUniformProvider for GPUTexture2dView {
  type Node = ShaderTexture;
}

impl ShaderUniformProvider for GPUSamplerView {
  type Node = ShaderSampler;
}
