use std::sync::Arc;

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_lighting_ibl::*;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub fn use_ibl(qcx: &mut impl QueryGPUHookCx) -> Option<IBLLightingComponentProvider> {
  let (qcx, brdf_lut) = qcx.use_gpu_init(|cx| {
    let brdf_lut_bitmap_png = include_bytes!("./brdf_lut.png");

    // todo, use two channel 16 bit
    create_gpu_tex_from_png_buffer(cx, brdf_lut_bitmap_png, TextureFormat::Rgba8Unorm)
  });

  let intensity =
    qcx.use_uniform_buffers::<EntityHandle<SceneEntity>, IblShaderInfo>(|source, cx| {
      let diffuse_illuminance = global_watch()
        .watch::<SceneHDRxEnvBackgroundIntensity>()
        .collective_filter_map(|v| v)
        .into_query_update_uniform(offset_of!(IblShaderInfo, diffuse_illuminance), cx);
      let specular_illuminance = global_watch()
        .watch::<SceneHDRxEnvBackgroundIntensity>()
        .collective_filter_map(|v| v)
        .into_query_update_uniform(offset_of!(IblShaderInfo, specular_illuminance), cx);

      source
        .with_source(specular_illuminance)
        .with_source(diffuse_illuminance)
    });

  let prefiltered = qcx.use_gpu_general_query(|gpu| CubeMapWithPrefilter {
    inner: RwLock::new(gpu_texture_cubes(gpu, Default::default())),
    map: Default::default(),
    gpu: gpu.clone(),
  });

  qcx.when_render(|| IBLLightingComponentProvider {
    prefiltered: prefiltered.unwrap(),
    brdf_lut: brdf_lut.clone(),
    uniform: intensity.unwrap(),
    access: global_database().read_foreign_key::<SceneHDRxEnvBackgroundCubeMap>(),
  })
}

pub struct IBLLightingComponentProvider {
  access: ForeignKeyReadView<SceneHDRxEnvBackgroundCubeMap>,
  prefiltered: LockReadGuardHolder<
    FastHashMap<EntityHandle<SceneTextureCubeEntity>, PreFilterMapGenerationResult>,
  >,
  brdf_lut: GPU2DTextureView,
  uniform: LockReadGuardHolder<IBLUniforms>,
}

type IBLUniforms = UniformUpdateContainer<EntityHandle<SceneEntity>, IblShaderInfo>;
impl LightSystemSceneProvider for IBLLightingComponentProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let map = self.access.get(scene)?;
    Some(Box::new(IBLLightingComponent {
      prefiltered: self.prefiltered.get(&map).unwrap().clone(),
      brdf_lut: self.brdf_lut.clone(),
      uniform: self.uniform.get(&scene).unwrap().clone(),
    }))
  }
}

type CubeMaintainerInternal<K> = QueryMutationCollector<
  FastHashMap<K, ValueChange<GPUCubeTextureView>>,
  FastHashMap<K, GPUCubeTextureView>,
>;

pub struct CubeMapWithPrefilter<K> {
  inner: RwLock<MultiUpdateContainer<CubeMaintainerInternal<K>>>,
  map: Arc<RwLock<CubeMapResults<K>>>,
  gpu: GPU,
}

type CubeMapResults<K> = FastHashMap<K, PreFilterMapGenerationResult>;

impl<K: CKey> ReactiveGeneralQuery for CubeMapWithPrefilter<K> {
  type Output = LockReadGuardHolder<CubeMapResults<K>>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let mut inner = self.inner.write();
    inner.poll_update(cx);

    let delta = std::mem::take(&mut inner.target.delta);
    let mut map = self.map.write();

    let gpu = &self.gpu;
    let mut encoder = gpu.create_encoder();

    for (k, change) in delta.iter_key_value() {
      match change.clone() {
        ValueChange::Delta(v, _) => {
          let config = PreFilterMapGenerationConfig {
            specular_resolution: 256,
            specular_sample_count: 32,
            diffuse_sample_count: 32,
            diffuse_resolution: 128,
          };

          let result = generate_pre_filter_map(&mut encoder, gpu, v, config);

          map.insert(k.clone(), result);
        }
        ValueChange::Remove(_) => {
          map.remove(&k);
        }
      }
    }

    gpu.submit_encoder(encoder);
    drop(map);

    self.map.make_read_holder()
  }
}
