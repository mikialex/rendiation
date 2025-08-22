use std::sync::Arc;

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_lighting_ibl::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_ibl(cx: &mut QueryGPUHookCx) -> Option<IBLLightingComponentProvider> {
  let (cx, brdf_lut) = cx.use_gpu_init(|cx| {
    let brdf_lut_bitmap_png = include_bytes!("./brdf_lut.png");

    // todo, use two channel 16 bit
    create_gpu_tex_from_png_buffer(cx, brdf_lut_bitmap_png, TextureFormat::Rgba8Unorm)
  });

  let intensity = cx.use_uniform_buffers();
  cx.use_changes::<SceneHDRxEnvBackgroundIntensity>()
    .filter_map_changes(|v| v)
    .update_uniforms(
      &intensity,
      offset_of!(IblShaderInfo, diffuse_illuminance),
      cx.gpu,
    );

  cx.use_changes::<SceneHDRxEnvBackgroundIntensity>()
    .filter_map_changes(|v| v)
    .update_uniforms(
      &intensity,
      offset_of!(IblShaderInfo, specular_illuminance),
      cx.gpu,
    );

  let prefiltered = use_prefilter_cube_maps(cx);

  cx.when_render(|| IBLLightingComponentProvider {
    prefiltered: prefiltered.make_read_holder(),
    brdf_lut: brdf_lut.clone(),
    uniform: intensity.make_read_holder(),
    access: global_database().read_foreign_key::<SceneHDRxEnvBackgroundCubeMap>(),
  })
}

pub struct IBLLightingComponentProvider {
  access: ForeignKeyReadView<SceneHDRxEnvBackgroundCubeMap>,
  prefiltered: LockReadGuardHolder<FastHashMap<RawEntityHandle, PreFilterMapGenerationResult>>,
  brdf_lut: GPU2DTextureView,
  uniform: LockReadGuardHolder<IBLUniforms>,
}

type IBLUniforms = UniformBufferCollectionRaw<u32, IblShaderInfo>;
impl LightSystemSceneProvider for IBLLightingComponentProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let map = self.access.get(scene)?;
    Some(Box::new(IBLLightingComponent {
      prefiltered: self.prefiltered.get(&map.into_raw()).unwrap().clone(),
      brdf_lut: self.brdf_lut.clone(),
      uniform: self.uniform.get(&scene.alloc_index()).unwrap().clone(),
    }))
  }
}

type CubeMapResults = FastHashMap<RawEntityHandle, PreFilterMapGenerationResult>;

pub fn use_prefilter_cube_maps(cx: &mut QueryGPUHookCx) -> Arc<RwLock<CubeMapResults>> {
  let (env_background_map_gpu, changes) = use_gpu_texture_cubes(cx, false);

  let (cx, _cube_map) = cx.use_plain_state(|| Arc::new(RwLock::new(CubeMapResults::default())));

  let mut cube_map = _cube_map.write();
  for k in changes.removed_keys {
    cube_map.remove(&k);
  }

  let config = PreFilterMapGenerationConfig {
    specular_resolution: 256,
    specular_sample_count: 32,
    diffuse_sample_count: 32,
    diffuse_resolution: 128,
  };

  if !changes.changed_keys.is_empty() {
    let mut encoder = cx.gpu.create_encoder();
    let cubes = env_background_map_gpu.read();

    for k in changes.changed_keys.iter() {
      let cube = cubes.get(k).unwrap();

      let result = generate_pre_filter_map(&mut encoder, cx.gpu, cube, &config);
      cube_map.insert(*k, result);
    }

    cx.gpu.submit_encoder(encoder);
  }

  _cube_map.clone()
}
