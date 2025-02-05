use rendiation_texture_core::CubeTextureFace;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

fn cube_face_update<FK, T>(face: CubeTextureFace, cx: &GPU) -> impl QueryBasedUpdate<T>
where
  FK: ForeignKeySemantic<Entity = SceneTextureCubeEntity, ForeignEntity = SceneTexture2dEntity>,
  T: QueryLikeMutateTarget<EntityHandle<SceneTextureCubeEntity>, GPUCubeTextureView>,
{
  global_watch()
    .watch::<SceneTexture2dEntityDirectContent>()
    .collective_filter_map(|v| v)
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<FK>())
    .into_query_update_cube_face(face, cx, false)
}

pub fn gpu_texture_cubes<T>(cx: &GPU, init: T) -> MultiUpdateContainer<T>
where
  T: QueryLikeMutateTarget<EntityHandle<SceneTextureCubeEntity>, GPUCubeTextureView> + 'static,
{
  let px = cube_face_update::<SceneTextureCubeXPositiveFace, T>(CubeTextureFace::PositiveX, cx);
  let py = cube_face_update::<SceneTextureCubeYPositiveFace, T>(CubeTextureFace::PositiveY, cx);
  let pz = cube_face_update::<SceneTextureCubeZPositiveFace, T>(CubeTextureFace::PositiveZ, cx);
  let nx = cube_face_update::<SceneTextureCubeXNegativeFace, T>(CubeTextureFace::NegativeX, cx);
  let ny = cube_face_update::<SceneTextureCubeYNegativeFace, T>(CubeTextureFace::NegativeY, cx);
  let nz = cube_face_update::<SceneTextureCubeZNegativeFace, T>(CubeTextureFace::NegativeZ, cx);

  MultiUpdateContainer::<T>::new(init)
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
      usage: TextureUsages::all() - TextureUsages::STORAGE_ATOMIC,
    },
    device,
  )
  .try_into()
  .unwrap()
}
