use rendiation_shader_api::*;

use crate::*;

pub struct BindingResourceArray<T> {
  bindings: Arc<Vec<T>>,
  resource_id: usize,
}

impl<T> Clone for BindingResourceArray<T> {
  fn clone(&self) -> Self {
    Self {
      bindings: self.bindings.clone(),
      resource_id: self.resource_id,
    }
  }
}

impl<T> Default for BindingResourceArray<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T> BindingResourceArray<T> {
  pub fn new(bindings: Arc<Vec<T>>) -> Self {
    Self {
      bindings,
      resource_id: get_new_resource_guid(),
    }
  }
}

impl CacheAbleBindingSource for BindingResourceArray<GPUTextureView> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::TextureViewArray(self.bindings.clone()),
      view_id: self.resource_id,
    }
  }
}

// todo, improve for performance and impl for other strong typed texture type
impl CacheAbleBindingSource for BindingResourceArray<GPU2DTextureView> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.0.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::TextureViewArray(Arc::new(lowered)),
      view_id: self.resource_id,
    }
  }
}

impl CacheAbleBindingSource for BindingResourceArray<GPUSamplerView> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::SamplerArray(self.bindings.clone()),
      view_id: self.resource_id,
    }
  }
}

// todo, improve for performance and impl for other strong type
impl<T: ?Sized + Std430MaybeUnsized> CacheAbleBindingSource
  for BindingResourceArray<StorageBufferDataView<T>>
{
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.gpu.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::BufferArray(Arc::new(lowered)),
      view_id: self.resource_id,
    }
  }
}
// todo, improve for performance and impl for other strong type
impl<T: ?Sized + Std430MaybeUnsized> CacheAbleBindingSource
  for BindingResourceArray<StorageBufferReadOnlyDataView<T>>
{
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.gpu.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::BufferArray(Arc::new(lowered)),
      view_id: self.resource_id,
    }
  }
}

/// the binding array length is inject into shader, so we have to impl shader hash for it.
impl<T: 'static> ShaderHashProvider for BindingResourceArray<T> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.bindings.len().hash(hasher)
  }
}

impl<T> ShaderBindingProvider for BindingResourceArray<T>
where
  T: ShaderBindingProvider,
  ShaderHandlePtr<BindingArray<T::Node>>: ShaderNodeType,
{
  type Node = ShaderHandlePtr<BindingArray<T::Node>>;

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::TYPE;

    if let ShaderValueType::BindingArray { count, .. } = &mut ty {
      *count = self.bindings.len();
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty: Self::Node::TYPE,
    }
  }
}
