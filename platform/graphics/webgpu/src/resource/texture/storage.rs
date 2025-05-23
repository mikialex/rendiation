use crate::*;

impl<D, F> GPUTypedTextureView<D, F> {
  pub fn into_storage_texture_view<A>(self) -> Option<StorageTextureView<A, D, F>> {
    if !self
      .resource
      .desc
      .usage
      .contains(TextureUsages::STORAGE_BINDING)
    {
      return None;
    }

    if self.resource.desc.sample_count > 1 {
      return None;
    }

    StorageFormat::try_from(self.resource.desc.format)
      .ok()
      .map(|format| StorageTextureView {
        texture: self,
        access: PhantomData,
        format,
      })
  }
  pub fn into_storage_texture_view_readwrite(
    self,
  ) -> Option<StorageTextureView<StorageTextureAccessReadWrite, D, F>> {
    self.into_storage_texture_view::<StorageTextureAccessReadWrite>()
  }
  pub fn into_storage_texture_view_readonly(
    self,
  ) -> Option<StorageTextureView<StorageTextureAccessReadonly, D, F>> {
    self.into_storage_texture_view::<StorageTextureAccessReadonly>()
  }
  pub fn into_storage_texture_view_writeonly(
    self,
  ) -> Option<StorageTextureView<StorageTextureAccessWriteonly, D, F>> {
    self.into_storage_texture_view::<StorageTextureAccessWriteonly>()
  }
}

pub type StorageTextureViewReadWrite2D<F = f32> =
  StorageTextureView<StorageTextureAccessReadWrite, TextureDimension2, F>;
pub type StorageTextureViewReadonly2D<F = f32> =
  StorageTextureView<StorageTextureAccessReadonly, TextureDimension2, F>;
pub type StorageTextureViewWriteonly2D<F = f32> =
  StorageTextureView<StorageTextureAccessWriteonly, TextureDimension2, F>;

pub struct StorageTextureView<A, D, F> {
  pub texture: GPUTypedTextureView<D, F>,
  access: PhantomData<A>,
  format: StorageFormat,
}

impl<A, D, F> Clone for StorageTextureView<A, D, F> {
  fn clone(&self) -> Self {
    Self {
      texture: self.texture.clone(),
      access: self.access,
      format: self.format,
    }
  }
}

impl<A, D, F> CacheAbleBindingSource for StorageTextureView<A, D, F> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

impl<A, D, F> ShaderBindingProvider for StorageTextureView<A, D, F>
where
  A: StorageTextureAccessMarker,
  D: ShaderTextureDimension,
  F: ShaderTextureKind,
{
  /// note: multi sampled F is not valid, but we have already rejected at runtime.
  /// so this is sound.
  type Node = ShaderBinding<ShaderStorageTexture<A, D, F>>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::ty();

    if let ShaderValueType::Single(ShaderValueSingleType::StorageTexture { format, .. }) = &mut ty {
      *format = self.format;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty,
    }
  }
}
