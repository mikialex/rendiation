use rendiation_shader_api::{BindingArray, ShaderBindingProvider, ShaderNodeSingleType};

use crate::*;

pub struct BindingResourceArray<T, const N: usize> {
  bindings: Arc<Vec<T>>,
  resource_id: usize,
}

impl<T, const N: usize> Default for BindingResourceArray<T, N> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T, const N: usize> BindingResourceArray<T, N> {
  pub fn new(bindings: Arc<Vec<T>>) -> Self {
    Self {
      bindings,
      resource_id: get_new_resource_guid(),
    }
  }
}

impl<const N: usize> CacheAbleBindingSource for BindingResourceArray<GPUTextureView, N> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::TextureViewArray(self.bindings.clone()),
      view_id: self.resource_id,
    }
  }
}

// todo, improve for performance and impl for other strong typed texture type
impl<const N: usize> CacheAbleBindingSource for BindingResourceArray<GPU2DTextureView, N> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.0.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::TextureViewArray(Arc::new(lowered)),
      view_id: self.resource_id,
    }
  }
}

impl<const N: usize> CacheAbleBindingSource for BindingResourceArray<GPUSamplerView, N> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::SamplerArray(self.bindings.clone()),
      view_id: self.resource_id,
    }
  }
}

impl<T, const N: usize> ShaderBindingProvider for BindingResourceArray<T, N>
where
  T: ShaderBindingProvider,
  T::Node: ShaderNodeSingleType,
{
  type Node = BindingArray<T::Node, N>;
}
