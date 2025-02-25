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
  fn into_storage_texture_view_readonly(self) -> Option<StorageTextureViewReadonly<Self>>;
  fn into_storage_texture_view_writeonly(self) -> Option<StorageTextureViewWriteOnly<Self>>;
  fn into_storage_texture_view_readwrite(self) -> Option<StorageTextureViewReadWrite<Self>>;
}

impl<T: StorageShaderTypeMapping> IntoStorageTextureView for T {
  fn into_storage_texture_view_readonly(self) -> Option<StorageTextureViewReadonly<Self>> {
    self.downcast().map(|format| StorageTextureViewReadonly {
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

  fn into_storage_texture_view_readwrite(self) -> Option<StorageTextureViewReadWrite<Self>> {
    self.downcast().map(|format| StorageTextureViewReadWrite {
      texture: self,
      format,
    })
  }
}

pub struct StorageTextureViewReadonly<T> {
  texture: T,
  format: StorageFormat,
}

impl<T: CacheAbleBindingSource> CacheAbleBindingSource for StorageTextureViewReadonly<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

impl<T: StorageShaderTypeMapping> ShaderBindingProvider for StorageTextureViewReadonly<T> {
  type Node = ShaderBinding<T::StorageTextureShaderTypeR>;
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
      *access = rendiation_shader_api::StorageTextureAccess::Load;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty,
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
  type Node = ShaderBinding<T::StorageTextureShaderTypeW>;
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
      *access = rendiation_shader_api::StorageTextureAccess::Store;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty,
    }
  }
}

#[derive(Clone)]
pub struct StorageTextureViewReadWrite<T> {
  pub texture: T,
  pub format: StorageFormat,
}

impl<T: CacheAbleBindingSource> CacheAbleBindingSource for StorageTextureViewReadWrite<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.texture.get_binding_build_source()
  }
}

impl<T: StorageShaderTypeMapping> ShaderBindingProvider for StorageTextureViewReadWrite<T> {
  type Node = ShaderBinding<T::StorageTextureShaderTypeRW>;
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
      *access = rendiation_shader_api::StorageTextureAccess::LoadStore;
    }

    ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: false,
      writeable_if_storage: false,
      ty,
    }
  }
}
