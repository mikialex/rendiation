use rendiation_shader_api::*;

use crate::*;

pub struct BindingResourceArray<T> {
  bindings: Arc<Vec<T>>,
  max_binding_length: u32,
  /// note, here we using a new resource id to represent the all view id in bindings to
  /// reduce the binding time id relation maintain and hashing cost
  /// this is ok to do so because the binding is immutable.
  pseudo_view_id: usize,
  _binding_dropper: Arc<BindGroupCacheInvalidation>,
}

impl<T> Clone for BindingResourceArray<T> {
  fn clone(&self) -> Self {
    Self {
      bindings: self.bindings.clone(),
      max_binding_length: self.max_binding_length,
      pseudo_view_id: self.pseudo_view_id,
      _binding_dropper: self._binding_dropper.clone(),
    }
  }
}

impl<T> BindingResourceArray<T> {
  pub fn new(bindings: Arc<Vec<T>>, max_binding_length: u32, device: &GPUDevice) -> Self {
    assert!(max_binding_length >= bindings.len() as u32);
    let pseudo_view_id = get_new_resource_guid();
    Self {
      bindings,
      max_binding_length,
      pseudo_view_id,
      _binding_dropper: Arc::new(device.get_binding_cache().create_dropper(pseudo_view_id)),
    }
  }
}

impl CacheAbleBindingSource for BindingResourceArray<GPUTextureView> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::TextureViewArray(self.bindings.clone()),
      view_id: self.pseudo_view_id,
    }
  }
}

// todo, improve for performance
impl<D, F> CacheAbleBindingSource for BindingResourceArray<GPUTypedTextureView<D, F>> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.texture.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::TextureViewArray(Arc::new(lowered)),
      view_id: self.pseudo_view_id,
    }
  }
}

impl CacheAbleBindingSource for BindingResourceArray<GPUSamplerView> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::SamplerArray(self.bindings.clone()),
      view_id: self.pseudo_view_id,
    }
  }
}

// todo, improve for performance
impl<T: ?Sized + Std430MaybeUnsized> CacheAbleBindingSource
  for BindingResourceArray<StorageBufferDataView<T>>
{
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.gpu.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::BufferArray(Arc::new(lowered)),
      view_id: self.pseudo_view_id,
    }
  }
}
// todo, improve for performance
impl<T: ?Sized + Std430MaybeUnsized> CacheAbleBindingSource
  for BindingResourceArray<StorageBufferReadonlyDataView<T>>
{
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    let lowered = self.bindings.iter().map(|v| v.gpu.clone()).collect();
    CacheAbleBindingBuildSource {
      source: BindingResourceOwned::BufferArray(Arc::new(lowered)),
      view_id: self.pseudo_view_id,
    }
  }
}

/// the binding array length is inject into shader, so we have to impl shader hash for it.
impl<T: 'static> ShaderHashProvider for BindingResourceArray<T> {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.max_binding_length.hash(hasher)
  }
}

impl<T> ShaderBindingProvider for BindingResourceArray<T>
where
  T: ShaderBindingProvider,
  ShaderBinding<BindingArray<T::Node>>: ShaderNodeType,
{
  type Node = ShaderBinding<BindingArray<T::Node>>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::ty();

    if let ShaderValueType::BindingArray { count, .. } = &mut ty {
      *count = self.max_binding_length as usize;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty,
    }
  }
}
