use shadergraph::{BindingArray, ShaderBindingProvider, ShaderGraphNodeSingleType};

use crate::*;

pub struct BindingResourceArray<T, const N: usize> {
  bindings: Arc<Vec<T>>,
  resource_id: usize,
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

impl<T, const N: usize> ShaderBindingProvider for BindingResourceArray<T, N>
where
  T: ShaderBindingProvider,
  T::Node: ShaderGraphNodeSingleType,
{
  type Node = BindingArray<T::Node, N>;
}
