use shadergraph::*;

pub use crate::*;

impl<T: ShaderGraphNodeType + Std140> ShaderUniformProvider for UniformBufferDataView<T> {
  type Node = T;
}

impl ShaderUniformProvider for GPUTexture2dView {
  type Node = ShaderTexture2D;
}

impl ShaderUniformProvider for GPUTextureCubeView {
  type Node = ShaderTextureCube;
}

impl ShaderUniformProvider for GPUSamplerView {
  type Node = ShaderSampler;
}

impl ShaderUniformProvider for GPUComparisonSamplerView {
  type Node = ShaderCompareSampler;
}
