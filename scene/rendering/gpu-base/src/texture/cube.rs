use fast_hash_collection::FastHashMap;
use rendiation_texture_core::CubeTextureFace;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

fn cube_face_update<FK>(
  face: CubeTextureFace,
  cx: &GPU,
) -> impl CollectionUpdate<FastHashMap<EntityHandle<SceneTextureCubeEntity>, GPUCubeTextureView>>
where
  FK: ForeignKeySemantic<Entity = SceneTextureCubeEntity, ForeignEntity = SceneTexture2dEntity>,
{
  global_watch()
    .watch::<SceneTexture2dEntityDirectContent>()
    .collective_filter_map(|v| v)
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<FK>())
    .into_cube_face_collection_update(face, cx)
}

pub fn gpu_texture_cubes(cx: &GPU) -> CubeMapUpdateContainer<EntityHandle<SceneTextureCubeEntity>> {
  let px = cube_face_update::<SceneTextureCubeXPositiveFace>(CubeTextureFace::PositiveX, cx);
  let py = cube_face_update::<SceneTextureCubeXPositiveFace>(CubeTextureFace::PositiveY, cx);
  let pz = cube_face_update::<SceneTextureCubeXPositiveFace>(CubeTextureFace::PositiveZ, cx);
  let nx = cube_face_update::<SceneTextureCubeXPositiveFace>(CubeTextureFace::NegativeX, cx);
  let ny = cube_face_update::<SceneTextureCubeXPositiveFace>(CubeTextureFace::NegativeY, cx);
  let nz = cube_face_update::<SceneTextureCubeXPositiveFace>(CubeTextureFace::NegativeZ, cx);

  CubeMapUpdateContainer::default()
    .with_source(px)
    .with_source(py)
    .with_source(pz)
    .with_source(nx)
    .with_source(ny)
    .with_source(nz)
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
      usage: TextureUsages::all(),
    },
    device,
  )
  .try_into()
  .unwrap()
}
