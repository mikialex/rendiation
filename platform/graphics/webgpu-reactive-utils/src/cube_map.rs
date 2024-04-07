use std::ops::Deref;

use fast_hash_collection::FastHashMap;
use rendiation_texture::{CubeTextureFace, GPUBufferImage};
use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

pub type CubeMapUpdateContainer<K> =
  MultiUpdateContainer<FastHashMap<AllocIdx<K>, GPUCubeTextureView>>;

pub struct CubeMapCollectionUpdate<T, K, V> {
  face: CubeTextureFace,
  upstream: T,
  phantom: PhantomData<(K, V)>,
  gpu_ctx: GPUResourceCtx,
}

pub trait CubeMapCollectionUpdateExt<K, V>: Sized {
  fn into_cube_face_collection_update(
    self,
    face: CubeTextureFace,
    gpu_ctx: &GPUResourceCtx,
  ) -> CubeMapCollectionUpdate<Self, K, V>;
}
impl<K, V, T> CubeMapCollectionUpdateExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue + Deref<Target = GPUBufferImage>,
{
  fn into_cube_face_collection_update(
    self,
    face: CubeTextureFace,
    gpu_ctx: &GPUResourceCtx,
  ) -> CubeMapCollectionUpdate<Self, K, V> {
    CubeMapCollectionUpdate {
      face,
      upstream: self,
      phantom: PhantomData,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<C, K, V> CollectionUpdate<FastHashMap<K, GPUCubeTextureView>>
  for CubeMapCollectionUpdate<C, K, V>
where
  V: CValue + Deref<Target = GPUBufferImage>,
  K: CKey,
  C: ReactiveCollection<K, V>,
{
  fn update_target(&mut self, target: &mut FastHashMap<K, GPUCubeTextureView>, cx: &mut Context) {
    if let Poll::Ready(changes) = self.upstream.poll_changes(cx) {
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
}
