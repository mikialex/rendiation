use crate::*;

pub struct ShaderStorageTextureR1D;
pub struct ShaderStorageTextureRW1D;
pub struct ShaderStorageTextureW1D;

pub struct ShaderStorageTextureR2D;
pub struct ShaderStorageTextureRW2D;
pub struct ShaderStorageTextureW2D;

pub struct ShaderStorageTextureR3D;
pub struct ShaderStorageTextureRW3D;
pub struct ShaderStorageTextureW3D;

pub struct ShaderStorageTextureR2DArray;
pub struct ShaderStorageTextureRW2DArray;
pub struct ShaderStorageTextureW2DArray;

#[macro_export]
macro_rules! storage_tex_impl {
  ($ty: ty, $ty_value: expr) => {
    sg_node_impl!(
      $ty,
      ShaderValueSingleType::StorageTexture {
        dimension: $ty_value,
        format: StorageFormat::R8Unorm, // this will be override by container instance.
        access: StorageTextureAccess::Load,
      }
    );
  };
}

storage_tex_impl!(ShaderStorageTextureR1D, TextureViewDimension::D1);
storage_tex_impl!(ShaderStorageTextureRW1D, TextureViewDimension::D1);
storage_tex_impl!(ShaderStorageTextureW1D, TextureViewDimension::D1);

impl D1TextureType for ShaderStorageTextureR1D {}
impl D1TextureType for ShaderStorageTextureRW1D {}
impl D1TextureType for ShaderStorageTextureW1D {}

storage_tex_impl!(ShaderStorageTextureR2D, TextureViewDimension::D2);
storage_tex_impl!(ShaderStorageTextureRW2D, TextureViewDimension::D2);
storage_tex_impl!(ShaderStorageTextureW2D, TextureViewDimension::D2);

impl D2LikeTextureType for ShaderStorageTextureR2D {}
impl D2LikeTextureType for ShaderStorageTextureRW2D {}
impl D2LikeTextureType for ShaderStorageTextureW2D {}

storage_tex_impl!(ShaderStorageTextureR3D, TextureViewDimension::D3);
storage_tex_impl!(ShaderStorageTextureRW3D, TextureViewDimension::D3);
storage_tex_impl!(ShaderStorageTextureW3D, TextureViewDimension::D3);

impl D3TextureType for ShaderStorageTextureR3D {}
impl D3TextureType for ShaderStorageTextureRW3D {}
impl D3TextureType for ShaderStorageTextureW3D {}

storage_tex_impl!(ShaderStorageTextureR2DArray, TextureViewDimension::D2Array);
storage_tex_impl!(ShaderStorageTextureRW2DArray, TextureViewDimension::D2Array);
storage_tex_impl!(ShaderStorageTextureW2DArray, TextureViewDimension::D2Array);

impl D2LikeTextureType for ShaderStorageTextureR2DArray {}
impl D2LikeTextureType for ShaderStorageTextureRW2DArray {}
impl D2LikeTextureType for ShaderStorageTextureW2DArray {}

pub trait ShaderStorageTextureLike {}

impl ShaderStorageTextureLike for ShaderStorageTextureR1D {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW1D {}
impl ShaderStorageTextureLike for ShaderStorageTextureW1D {}

impl ShaderStorageTextureLike for ShaderStorageTextureR2D {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW2D {}
impl ShaderStorageTextureLike for ShaderStorageTextureW2D {}

impl ShaderStorageTextureLike for ShaderStorageTextureR3D {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW3D {}
impl ShaderStorageTextureLike for ShaderStorageTextureW3D {}

impl ShaderStorageTextureLike for ShaderStorageTextureR2DArray {}
impl ShaderStorageTextureLike for ShaderStorageTextureRW2DArray {}
impl ShaderStorageTextureLike for ShaderStorageTextureW2DArray {}

impl SingleLayerTarget for ShaderStorageTextureR1D {}
impl SingleLayerTarget for ShaderStorageTextureRW1D {}
impl SingleLayerTarget for ShaderStorageTextureW1D {}

impl SingleLayerTarget for ShaderStorageTextureR2D {}
impl SingleLayerTarget for ShaderStorageTextureRW2D {}
impl SingleLayerTarget for ShaderStorageTextureW2D {}

impl SingleLayerTarget for ShaderStorageTextureR3D {}
impl SingleLayerTarget for ShaderStorageTextureRW3D {}
impl SingleLayerTarget for ShaderStorageTextureW3D {}

impl ArrayLayerTarget for ShaderStorageTextureR2DArray {}
impl ArrayLayerTarget for ShaderStorageTextureRW2DArray {}
impl ArrayLayerTarget for ShaderStorageTextureW2DArray {}

impl SingleSampleTarget for ShaderStorageTextureR1D {}
impl SingleSampleTarget for ShaderStorageTextureRW1D {}
impl SingleSampleTarget for ShaderStorageTextureW1D {}
impl SingleSampleTarget for ShaderStorageTextureR2D {}
impl SingleSampleTarget for ShaderStorageTextureRW2D {}
impl SingleSampleTarget for ShaderStorageTextureW2D {}
impl SingleSampleTarget for ShaderStorageTextureR3D {}
impl SingleSampleTarget for ShaderStorageTextureRW3D {}
impl SingleSampleTarget for ShaderStorageTextureW3D {}
impl SingleSampleTarget for ShaderStorageTextureR2DArray {}
impl SingleSampleTarget for ShaderStorageTextureRW2DArray {}
impl SingleSampleTarget for ShaderStorageTextureW2DArray {}

impl<T> BindingNode<T>
where
  T: ShaderTextureType + ShaderStorageTextureLike + ShaderDirectLoad + SingleLayerTarget,
{
  pub fn write_texel(&self, at: Node<T::LoadInput>, tex: Node<T::Output>) {
    call_shader_api(|api| {
      api.texture_store(ShaderTextureStore {
        image: self.handle(),
        position: at.handle(),
        array_index: None,
        value: tex.handle(),
      })
    })
  }
}

impl<T> BindingNode<T>
where
  T: ShaderTextureType + ShaderStorageTextureLike + ShaderDirectLoad + ArrayLayerTarget,
{
  pub fn write_texel_index(&self, at: Node<T::LoadInput>, index: Node<u32>, tex: Node<T::Output>) {
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
