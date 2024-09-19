use crate::*;

pub trait StorageShaderTypeMapping {
  type StorageTextureShaderTypeR: ShaderNodeType;
  type StorageTextureShaderTypeRW: ShaderNodeType;
  type StorageTextureShaderTypeW: ShaderNodeType;
  fn downcast(&self) -> Option<StorageFormat>;
}

impl StorageShaderTypeMapping for GPU1DTextureView {
  type StorageTextureShaderTypeR = ShaderStorageTextureR1D;
  type StorageTextureShaderTypeRW = ShaderStorageTextureRW1D;
  type StorageTextureShaderTypeW = ShaderStorageTextureW1D;
  fn downcast(&self) -> Option<StorageFormat> {
    if !self
      .resource
      .desc
      .usage
      .contains(TextureUsages::STORAGE_BINDING)
    {
      return None;
    }
    StorageFormat::try_from(self.resource.desc.format).ok()
  }
}
impl StorageShaderTypeMapping for GPU2DTextureView {
  type StorageTextureShaderTypeR = ShaderStorageTextureR2D;
  type StorageTextureShaderTypeRW = ShaderStorageTextureRW2D;
  type StorageTextureShaderTypeW = ShaderStorageTextureW2D;
  fn downcast(&self) -> Option<StorageFormat> {
    if !self
      .resource
      .desc
      .usage
      .contains(TextureUsages::STORAGE_BINDING)
    {
      return None;
    }
    StorageFormat::try_from(self.resource.desc.format).ok()
  }
}

impl StorageShaderTypeMapping for GPU2DArrayTextureView {
  type StorageTextureShaderTypeR = ShaderStorageTextureR2DArray;
  type StorageTextureShaderTypeRW = ShaderStorageTextureRW2DArray;
  type StorageTextureShaderTypeW = ShaderStorageTextureW2DArray;
  fn downcast(&self) -> Option<StorageFormat> {
    if !self
      .resource
      .desc
      .usage
      .contains(TextureUsages::STORAGE_BINDING)
    {
      return None;
    }
    StorageFormat::try_from(self.resource.desc.format).ok()
  }
}

impl StorageShaderTypeMapping for GPU3DTextureView {
  type StorageTextureShaderTypeR = ShaderStorageTextureR3D;
  type StorageTextureShaderTypeRW = ShaderStorageTextureRW3D;
  type StorageTextureShaderTypeW = ShaderStorageTextureW3D;
  fn downcast(&self) -> Option<StorageFormat> {
    if !self
      .resource
      .desc
      .usage
      .contains(TextureUsages::STORAGE_BINDING)
    {
      return None;
    }
    StorageFormat::try_from(self.resource.desc.format).ok()
  }
}

pub trait IntoStorageTextureView: Sized {
  fn into_storage_texture_view_readonly(self) -> Option<StorageTextureViewReadOnly<Self>>;
  fn into_storage_texture_view_writeonly(self) -> Option<StorageTextureViewWriteOnly<Self>>;
  fn into_storage_texture_view_readwrite(self) -> Option<StorageTextureReadWrite<Self>>;
}

impl<T: StorageShaderTypeMapping> IntoStorageTextureView for T {
  fn into_storage_texture_view_readonly(self) -> Option<StorageTextureViewReadOnly<Self>> {
    self.downcast().map(|format| StorageTextureViewReadOnly {
      texture: self,
      format,
    })
  }

  fn into_storage_texture_view_writeonly(self) -> Option<StorageTextureViewWriteOnly<Self>> {
    self.downcast().map(|format| StorageTextureViewWriteOnly {
      texture: self,
      format,
    })
  }

  fn into_storage_texture_view_readwrite(self) -> Option<StorageTextureReadWrite<Self>> {
    self.downcast().map(|format| StorageTextureReadWrite {
      texture: self,
      format,
    })
  }
}

pub struct StorageTextureViewReadOnly<T> {
  texture: T,
  format: StorageFormat,
}

impl<T: CacheAbleBindingSource> CacheAbleBindingSource for StorageTextureViewReadOnly<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

impl<T: StorageShaderTypeMapping> ShaderBindingProvider for StorageTextureViewReadOnly<T> {
  type Node = ShaderHandlePtr<T::StorageTextureShaderTypeR>;

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::ty();

    if let ShaderValueType::Single(ShaderValueSingleType::StorageTexture { format, .. }) = &mut ty {
      *format = self.format;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty: Self::Node::ty(),
    }
  }
}

pub struct StorageTextureViewWriteOnly<T> {
  pub texture: T,
  pub format: StorageFormat,
}

impl<T: CacheAbleBindingSource> CacheAbleBindingSource for StorageTextureViewWriteOnly<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

impl<T: StorageShaderTypeMapping> ShaderBindingProvider for StorageTextureViewWriteOnly<T> {
  type Node = ShaderHandlePtr<T::StorageTextureShaderTypeW>;

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::ty();

    if let ShaderValueType::Single(ShaderValueSingleType::StorageTexture { format, .. }) = &mut ty {
      *format = self.format;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty: Self::Node::ty(),
    }
  }
}

pub struct StorageTextureReadWrite<T> {
  pub texture: T,
  pub format: StorageFormat,
}

impl<T: CacheAbleBindingSource> CacheAbleBindingSource for StorageTextureReadWrite<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

impl<T: StorageShaderTypeMapping> ShaderBindingProvider for StorageTextureReadWrite<T> {
  type Node = ShaderHandlePtr<T::StorageTextureShaderTypeRW>;

  fn binding_desc(&self) -> ShaderBindingDescriptor {
    let mut ty = Self::Node::ty();

    if let ShaderValueType::Single(ShaderValueSingleType::StorageTexture { format, .. }) = &mut ty {
      *format = self.format;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty: Self::Node::ty(),
    }
  }
}
