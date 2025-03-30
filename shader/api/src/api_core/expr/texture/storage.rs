use crate::*;

pub trait StorageTextureReadable {}
pub trait StorageTextureWriteable {}

pub struct ShaderStorageTexture<A, T>(A, T);
impl<A, T> ShaderNodeSingleType for ShaderStorageTexture<A, T>
where
  T: ShaderTextureDimension,
  A: 'static,
{
  fn single_ty() -> ShaderValueSingleType {
    ShaderValueSingleType::StorageTexture {
      dimension: T::DIMENSION,
      format: StorageFormat::R8Unorm, // this will be override by container instance.
      access: StorageTextureAccess::Load, // this will be override by container instance.
    }
  }
}

impl<A, T> ShaderNodeType for ShaderStorageTexture<A, T>
where
  T: ShaderTextureDimension,
  A: 'static,
{
  fn ty() -> ShaderValueType {
    ShaderValueType::Single(Self::single_ty())
  }
}

pub struct StorageTextureAccessReadonly;
impl StorageTextureReadable for StorageTextureAccessReadonly {}
pub struct StorageTextureAccessWriteonly;
impl StorageTextureWriteable for StorageTextureAccessWriteonly {}
pub struct StorageTextureAccessReadWrite;
impl StorageTextureReadable for StorageTextureAccessReadWrite {}
impl StorageTextureWriteable for StorageTextureAccessReadWrite {}

pub type ShaderStorageTextureR1D =
  ShaderStorageTexture<StorageTextureAccessReadonly, TextureDimension1>;
pub type ShaderStorageTextureRW1D =
  ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension1>;
pub type ShaderStorageTextureW1D =
  ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension1>;

pub type ShaderStorageTextureR2D =
  ShaderStorageTexture<StorageTextureAccessReadonly, TextureDimension2>;
pub type ShaderStorageTextureRW2D =
  ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension2>;
pub type ShaderStorageTextureW2D =
  ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2>;

pub type ShaderStorageTextureR3D =
  ShaderStorageTexture<StorageTextureAccessReadonly, TextureDimension3>;
pub type ShaderStorageTextureRW3D =
  ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension3>;
pub type ShaderStorageTextureW3D =
  ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension3>;

pub type ShaderStorageTextureR2DArray =
  ShaderStorageTexture<StorageTextureAccessReadonly, TextureDimension2Array>;
pub type ShaderStorageTextureRW2DArray =
  ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension2Array>;
pub type ShaderStorageTextureW2DArray =
  ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2Array>;

impl<A, D> BindingNode<ShaderStorageTexture<A, D>>
where
  D: ShaderTextureDimension,
{
  pub fn load_texel(&self, position: Node<TextureSampleInputOf<D, u32>>) -> Node<Vec4<f32>>
  where
    D: SingleLayerTarget,
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
  ) -> Node<Vec4<f32>>
  where
    D: ArrayLayerTarget,
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

  pub fn write_texel(&self, at: Node<TextureSampleInputOf<D, u32>>, tex: Node<Vec4<f32>>)
  where
    D: SingleLayerTarget,
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
    tex: Node<Vec4<f32>>,
  ) where
    D: ArrayLayerTarget,
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

impl<A, D> BindingNode<ShaderStorageTexture<A, D>>
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
