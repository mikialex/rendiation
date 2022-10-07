use shadergraph::*;

pub use crate::*;

impl<T: ShaderGraphNodeType + Std140> ShaderUniformProvider for UniformBufferDataView<T> {
  type Node = T;
}

impl ShaderUniformProvider for GPU2DTextureView {
  type Node = ShaderTexture2D;
}

impl ShaderUniformProvider for GPU2DArrayTextureView {
  type Node = ShaderTexture2DArray;
}

impl ShaderUniformProvider for GPUCubeTextureView {
  type Node = ShaderTextureCube;
}

impl ShaderUniformProvider for GPUSamplerView {
  type Node = ShaderSampler;
}

impl ShaderUniformProvider for GPUComparisonSamplerView {
  type Node = ShaderCompareSampler;
}
