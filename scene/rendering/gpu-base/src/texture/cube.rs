use std::sync::Arc;

use fast_hash_collection::{FastHashMap, FastHashSet};
use parking_lot::RwLock;
use rendiation_texture_core::CubeTextureFace;
use rendiation_webgpu_hook_utils::*;

use crate::*;

#[derive(Default)]
pub struct CubeMapChanges {
  pub changed_keys: FastHashSet<RawEntityHandle>,
  pub removed_keys: FastHashSet<RawEntityHandle>,
}

// todo, share resource between background and ibl
#[rustfmt::skip]
pub fn use_gpu_texture_cubes(
  cx: &mut QueryGPUHookCx,
  allocate_mipmap: bool,
) -> (Arc<RwLock<FastHashMap<RawEntityHandle, GPUCubeTextureView>>>, CubeMapChanges) {
  let (cx, env_background_map_gpu) = cx.use_plain_state_default::<Arc<RwLock<FastHashMap<RawEntityHandle, GPUCubeTextureView>>>>();

  let mut target = env_background_map_gpu.write();
  let target = &mut target;
  let mut changed_keys = Default::default();

  cx.skip_if_not_waked(|cx|{
    use_cube_face_update::<SceneTextureCubeXPositiveFace>(cx, CubeTextureFace::PositiveX, allocate_mipmap, target, &mut changed_keys);
    use_cube_face_update::<SceneTextureCubeYPositiveFace>(cx, CubeTextureFace::PositiveY, allocate_mipmap, target, &mut changed_keys);
    use_cube_face_update::<SceneTextureCubeZPositiveFace>(cx, CubeTextureFace::PositiveZ, allocate_mipmap, target, &mut changed_keys);
    use_cube_face_update::<SceneTextureCubeXNegativeFace>(cx, CubeTextureFace::NegativeX, allocate_mipmap, target, &mut changed_keys);
    use_cube_face_update::<SceneTextureCubeYNegativeFace>(cx, CubeTextureFace::NegativeY, allocate_mipmap, target, &mut changed_keys);
    use_cube_face_update::<SceneTextureCubeZNegativeFace>(cx, CubeTextureFace::NegativeZ, allocate_mipmap, target, &mut changed_keys);
  });


  (env_background_map_gpu.clone(), changed_keys)
}

#[inline(always)]
fn use_cube_face_update<FK>(
  cx: &mut QueryGPUHookCx,
  face: CubeTextureFace,
  allocate_mipmap: bool,
  target: &mut FastHashMap<RawEntityHandle, GPUCubeTextureView>,
  changed_keys: &mut CubeMapChanges,
) where
  FK: ForeignKeySemantic<Entity = SceneTextureCubeEntity, ForeignEntity = SceneTexture2dEntity>,
{
  let id = (FK::component_id(), FK::Entity::entity_id());
  use_cube_face_update_impl(cx, face, allocate_mipmap, target, changed_keys, id);
}

#[inline(never)]
fn use_cube_face_update_impl(
  cx: &mut QueryGPUHookCx,
  face: CubeTextureFace,
  allocate_mipmap: bool,
  target: &mut FastHashMap<RawEntityHandle, GPUCubeTextureView>,
  changed_keys: &mut CubeMapChanges,
  (cid, e_id): (ComponentId, EntityId),
) {
  let change = cx
    .use_dual_query::<SceneTexture2dEntityDirectContent>()
    .map(|v| v.filter_map(|v| v))
    .fanout(cx.use_db_rev_ref_tri_view_impl(cid, e_id), cx)
    .use_assure_result(cx)
    .into_delta_change();

  let flags = cx.gpu.info().downgrade_info.flags;

  if let Some(change) = change.if_ready() {
    for k in change.iter_removed() {
      target.remove(&k);
      changed_keys.removed_keys.insert(k);
    }

    for (k, source) in change.iter_update_or_insert() {
      changed_keys.changed_keys.insert(k);
      changed_keys.removed_keys.remove(&k);

      let source = GPUBufferImageForeignImpl { inner: &source };
      let mip = if allocate_mipmap {
        MipLevelCount::BySize
      } else {
        MipLevelCount::EmptyMipMap
      };
      let desc = source.create_cube_desc(mip, flags);

      let create = || {
        let gpu_texture = GPUTexture::create(desc, &cx.gpu.device);
        let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();
        gpu_texture
          .create_view(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
          })
          .try_into()
          .unwrap()
      };

      let gpu_texture = target.entry(k).or_insert_with(create.clone());
      let gpu_texture: GPUCubeTexture = gpu_texture.resource.clone().try_into().unwrap();

      // todo, we current not checking if all face has same size and fmt
      if gpu_texture.desc.format != source.inner.format
        || gpu_texture.desc.size.width != usize::from(source.inner.size.width) as u32
      {
        target.remove(&k);
      }

      // recreate
      let gpu_texture = target.entry(k).or_insert_with(create);
      let gpu_texture: GPUCubeTexture = gpu_texture.resource.clone().try_into().unwrap();

      let _ = gpu_texture.upload(&cx.gpu.queue, &source, face, 0);
    }
  }
}

pub fn create_fallback_empty_cube_texture(device: &GPUDevice) -> GPUCubeTexture {
  GPUTexture::create(
    TextureDescriptor {
      label: "global default texture".into(),
      size: Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 6,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8UnormSrgb,
      view_formats: &[],
      usage: basic_texture_usages(),
    },
    device,
  )
  .try_into()
  .unwrap()
}
