use std::ops::Deref;

use fast_hash_collection::FastHashMap;
use rendiation_texture_core::{CubeTextureFace, GPUBufferImage};
use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

pub type CubeMapUpdateContainer<K> = MultiUpdateContainer<FastHashMap<K, GPUCubeTextureView>>;

pub struct QueryBasedCubeMapUpdate<T> {
  face: CubeTextureFace,
  upstream: T,
  gpu_ctx: GPU,
}

pub trait CubeMapQueryUpdateExt: Sized {
  fn into_query_update_cube_face(
    self,
    face: CubeTextureFace,
    gpu_ctx: &GPU,
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
  ) -> QueryBasedCubeMapUpdate<Self> {
    QueryBasedCubeMapUpdate {
      face,
      upstream: self,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<C> QueryBasedUpdate<FastHashMap<C::Key, GPUCubeTextureView>> for QueryBasedCubeMapUpdate<C>
where
  C: ReactiveQuery,
  C::Value: Deref<Target = GPUBufferImage>,
{
  fn update_target(
    &mut self,
    target: &mut FastHashMap<C::Key, GPUCubeTextureView>,
    cx: &mut Context,
  ) {
    let (changes, _) = self.upstream.poll_changes(cx);

    for (k, v) in changes.iter_key_value() {
      let index = k;

      match v {
        ValueChange::Delta(v, _) => {
          let source: &GPUBufferImage = v.deref();

          let source = GPUBufferImageForeignImpl { inner: source };
          let desc = source.create_cube_desc(MipLevelCount::EmptyMipMap); // todo impl mipmap

          // todo, check desc is matched and recreated texture!
          let gpu_texture = target.entry(index).or_insert_with(|| {
            let gpu_texture = GPUTexture::create(desc, &self.gpu_ctx.device);
            let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();
            gpu_texture
              .create_view(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
              })
              .try_into()
              .unwrap()
          });

          let gpu_texture: GPUCubeTexture = gpu_texture.resource.clone().try_into().unwrap();
          let _ = gpu_texture.upload(&self.gpu_ctx.queue, &source, self.face, 0);
        }
        ValueChange::Remove(_) => {
          target.remove(&index);
        }
      }
    }
  }
}
