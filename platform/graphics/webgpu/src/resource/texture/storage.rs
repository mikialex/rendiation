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

pub type StorageTextureViewReadWrite2D =
  StorageTextureView<StorageTextureAccessReadWrite, TextureDimension2, f32>;
pub type StorageTextureViewReadonly2D =
  StorageTextureView<StorageTextureAccessReadonly, TextureDimension2, f32>;
pub type StorageTextureViewWriteonly2D =
  StorageTextureView<StorageTextureAccessWriteonly, TextureDimension2, f32>;

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
{
  type Node = ShaderBinding<ShaderStorageTexture<A, D>>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::ty();

    if let ShaderValueType::Single(ShaderValueSingleType::StorageTexture {
      format, access, ..
    }) = &mut ty
    {
      *format = self.format;
      *access = A::ACCESS;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty,
    }
  }
}
