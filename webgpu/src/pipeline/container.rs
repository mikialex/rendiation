use shadergraph::*;

pub use crate::*;

impl<T: ShaderGraphNodeType + Std140> ShaderUniformProvider for UniformBufferDataView<T> {
  type Node = T;
}

macro_rules! map_shader_ty {
  ($ty: ty, $shader_ty: ty) => {
    impl ShaderUniformProvider for $ty {
      type Node = $shader_ty;
    }
  };
}
map_shader_ty!(GPU1DTextureView, ShaderTexture1D);

map_shader_ty!(GPU2DTextureView, ShaderTexture2D);
map_shader_ty!(GPU2DArrayTextureView, ShaderTexture2DArray);

map_shader_ty!(GPUCubeTextureView, ShaderTextureCube);
map_shader_ty!(GPUCubeArrayTextureView, ShaderTextureCubeArray);

map_shader_ty!(GPU3DTextureView, ShaderTexture3D);

map_shader_ty!(GPU2DDepthTextureView, ShaderDepthTexture2D);
map_shader_ty!(GPU2DArrayDepthTextureView, ShaderDepthTexture2DArray);
map_shader_ty!(GPUCubeDepthTextureView, ShaderDepthTextureCube);
map_shader_ty!(GPUCubeArrayDepthTextureView, ShaderDepthTextureCubeArray);

map_shader_ty!(GPUSamplerView, ShaderSampler);
map_shader_ty!(GPUComparisonSamplerView, ShaderCompareSampler);
