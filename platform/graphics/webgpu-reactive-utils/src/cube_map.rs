use std::ops::Deref;

use rendiation_texture_core::{CubeTextureFace, GPUBufferImage};
use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

pub type CubeMapUpdateContainer<K> = MultiUpdateContainer<FastHashMap<K, GPUCubeTextureView>>;

pub struct QueryBasedCubeMapUpdate<T> {
  face: CubeTextureFace,
  upstream: T,
  gpu_ctx: GPU,
  allocate_mipmap: bool,
}

pub trait CubeMapQueryUpdateExt: Sized {
  fn into_query_update_cube_face(
    self,
    face: CubeTextureFace,
    gpu_ctx: &GPU,
    allocate_mipmap: bool,
  ) -> QueryBasedCubeMapUpdate<Self>;
}
impl<T> CubeMapQueryUpdateExt for T
where
  T: ReactiveQuery,
  T::Value: Deref<Target = GPUBufferImage>,
{
  fn into_query_update_cube_face(
    self,
    face: CubeTextureFace,
    gpu_ctx: &GPU,
    allocate_mipmap: bool,
  ) -> QueryBasedCubeMapUpdate<Self> {
    QueryBasedCubeMapUpdate {
      face,
      upstream: self,
      gpu_ctx: gpu_ctx.clone(),
      allocate_mipmap,
    }
  }
}

impl<C, T> QueryBasedUpdate<T> for QueryBasedCubeMapUpdate<C>
where
  C: ReactiveQuery,
  C::Value: Deref<Target = GPUBufferImage>,
  T: QueryLikeMutateTarget<C::Key, GPUCubeTextureView>,
{
  fn update_target(&mut self, target: &mut T, cx: &mut Context) {
    let (changes, _) = self.upstream.describe(cx).resolve_kept();

    for (k, v) in changes.iter_key_value() {
      let index = k;

      match v {
        ValueChange::Delta(v, _) => {
          let source: &GPUBufferImage = v.deref();

          let source = GPUBufferImageForeignImpl { inner: source };
          let mip = if self.allocate_mipmap {
            MipLevelCount::BySize
          } else {
            MipLevelCount::EmptyMipMap
          };
          let desc = source.create_cube_desc(mip);

          // todo, check desc is matched and recreated texture!
          if target.get_current(index.clone()).is_none() {
            let gpu_texture = GPUTexture::create(desc, &self.gpu_ctx.device);
            let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();
            let new = gpu_texture
              .create_view(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
              })
              .try_into()
              .unwrap();
            target.set_value(index.clone(), new);
          }

          let gpu_texture = target.get_current(index).unwrap();

          let gpu_texture: GPUCubeTexture = gpu_texture.resource.clone().try_into().unwrap();
          let _ = gpu_texture.upload(&self.gpu_ctx.queue, &source, self.face, 0);
        }
        ValueChange::Remove(_) => {
          target.remove(index);
        }
      }
    }
  }
}
