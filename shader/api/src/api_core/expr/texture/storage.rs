use crate::*;

pub trait StorageTextureAccessMarker: 'static {
  const ACCESS: StorageTextureAccess;
}

pub trait StorageTextureReadable: StorageTextureAccessMarker {}
pub trait StorageTextureWriteable: StorageTextureAccessMarker {}

pub struct ShaderStorageTexture<A, D, F>(A, D, F);
impl<A, D, F> ShaderNodeSingleType for ShaderStorageTexture<A, D, F>
where
  D: ShaderTextureDimension,
  A: StorageTextureAccessMarker,
  F: 'static,
{
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::StorageTexture {
      dimension: D::DIMENSION,
      format: StorageFormat::R8Unorm, // this will be override by container instance.
      access: A::ACCESS,
    }
  }
}

impl<A, D, F> ShaderNodeType for ShaderStorageTexture<A, D, F>
where
  D: ShaderTextureDimension,
  A: StorageTextureAccessMarker,
  F: 'static,
{
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

pub struct StorageTextureAccessReadonly;
impl StorageTextureAccessMarker for StorageTextureAccessReadonly {
  const ACCESS: StorageTextureAccess = StorageTextureAccess::Load;
}
impl StorageTextureReadable for StorageTextureAccessReadonly {}
pub struct StorageTextureAccessWriteonly;
impl StorageTextureAccessMarker for StorageTextureAccessWriteonly {
  const ACCESS: StorageTextureAccess = StorageTextureAccess::Store;
}
impl StorageTextureWriteable for StorageTextureAccessWriteonly {}
pub struct StorageTextureAccessReadWrite;
impl StorageTextureAccessMarker for StorageTextureAccessReadWrite {
  const ACCESS: StorageTextureAccess = StorageTextureAccess::LoadStore;
}
impl StorageTextureReadable for StorageTextureAccessReadWrite {}
impl StorageTextureWriteable for StorageTextureAccessReadWrite {}

// most used types:

pub type ShaderStorageTextureR2D<F = f32> =
  ShaderStorageTexture<StorageTextureAccessReadonly, TextureDimension2, F>;
pub type ShaderStorageTextureRW2D<F = f32> =
  ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension2, F>;
pub type ShaderStorageTextureW2D<F = f32> =
  ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2, F>;

impl<A, D, F> BindingNode<ShaderStorageTexture<A, D, F>>
where
  D: ShaderTextureDimension,
  Vec4<F>: ShaderNodeType,
{
  pub fn load_texel(&self, position: Node<TextureSampleInputOf<D, u32>>) -> Node<Vec4<F>>
  where
    D: SingleLayerTarget + DirectAccessTarget,
    A: StorageTextureReadable,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: None,
      sample_index: None,
      level: None, // must be none for storage texture
    })
    .insert_api()
  }

  pub fn load_texel_layer(
    &self,
    position: Node<TextureSampleInputOf<D, u32>>,
    layer: Node<u32>,
  ) -> Node<Vec4<F>>
  where
    D: ArrayLayerTarget + DirectAccessTarget,
    A: StorageTextureReadable,
  {
    ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
      texture: self.handle(),
      position: position.handle(),
      array_index: layer.handle().into(),
      sample_index: None,
      level: None, // must be none for storage texture
    })
    .insert_api()
  }

  pub fn write_texel(&self, at: Node<TextureSampleInputOf<D, u32>>, tex: Node<Vec4<F>>)
  where
    D: SingleLayerTarget + DirectAccessTarget,
    A: StorageTextureWriteable,
  {
    call_shader_api(|api| {
      api.texture_store(ShaderTextureStore {
        image: self.handle(),
        position: at.handle(),
        array_index: None,
        value: tex.handle(),
      })
    })
  }

  pub fn write_texel_index(
    &self,
    at: Node<TextureSampleInputOf<D, u32>>,
    index: Node<u32>,
    tex: Node<Vec4<F>>,
  ) where
    D: ArrayLayerTarget + DirectAccessTarget,
    A: StorageTextureWriteable,
  {
    call_shader_api(|api| {
      api.texture_store(ShaderTextureStore {
        image: self.handle(),
        position: at.handle(),
        array_index: Some(index.handle()),
        value: tex.handle(),
      })
    })
  }
}

impl<A, D, F> BindingNode<ShaderStorageTexture<A, D, F>>
where
  D: ShaderTextureDimension,
{
  pub fn texture_number_layers(&self) -> Node<u32>
  where
    D: ArrayLayerTarget + SingleSampleTarget,
  {
    ShaderNodeExpr::TextureQuery(self.handle(), TextureQuery::NumLayers).insert_api()
  }

  /// using None means base level
  fn texture_dimension(&self, level: Option<Node<u32>>) -> ShaderNodeExpr {
    ShaderNodeExpr::TextureQuery(
      self.handle(),
      TextureQuery::Size {
        level: level.map(|v| v.handle()),
      },
    )
  }

  /// using None means base level
  pub fn texture_dimension_1d(&self, level: Option<Node<u32>>) -> Node<u32>
  where
    D: D1LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  /// using None means base level
  pub fn texture_dimension_2d(&self, level: Option<Node<u32>>) -> Node<Vec2<u32>>
  where
    D: D2LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }

  /// using None means base level
  pub fn texture_dimension_3d(&self, level: Option<Node<u32>>) -> Node<Vec3<u32>>
  where
    D: D3LikeTextureType,
  {
    self.texture_dimension(level).insert_api()
  }
}
