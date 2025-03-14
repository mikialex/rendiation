use std::sync::Arc;

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_lighting_ibl::*;
use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub struct IBLProvider {
  brdf_lut: GPU2DTextureView,
  intensity: QueryToken,
  // todo
  // note, currently the cube map is standalone maintained, this is wasteful if user shared it elsewhere
  cube_map: QueryToken,
}

impl IBLProvider {
  pub fn new(cx: &GPU) -> Self {
    let brdf_lut_bitmap_png = include_bytes!("./brdf_lut.png");

    // todo, use two channel 16 bit
    let brdf_lut =
      create_gpu_tex_from_png_buffer(cx, brdf_lut_bitmap_png, TextureFormat::Rgba8Unorm);

    Self {
      brdf_lut,
      intensity: Default::default(),
      cube_map: Default::default(),
    }
  }
}

impl QueryBasedFeature<Box<dyn LightSystemSceneProvider>> for IBLProvider {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let diffuse_illuminance = global_watch()
      .watch::<SceneHDRxEnvBackgroundIntensity>()
      .collective_filter_map(|v| v)
      .into_query_update_uniform(offset_of!(IblShaderInfo, diffuse_illuminance), cx);
    let specular_illuminance = global_watch()
      .watch::<SceneHDRxEnvBackgroundIntensity>()
      .collective_filter_map(|v| v)
      .into_query_update_uniform(offset_of!(IblShaderInfo, specular_illuminance), cx);

    let intensity = UniformUpdateContainer::<EntityHandle<SceneEntity>, IblShaderInfo>::default()
      .with_source(specular_illuminance)
      .with_source(diffuse_illuminance);

    self.intensity = qcx.register_multi_updater(intensity);

    let cube_prefilter = CubeMapWithPrefilter {
      inner: RwLock::new(gpu_texture_cubes(cx, Default::default())),
      map: Default::default(),
      gpu: cx.clone(),
    };

    self.cube_map = qcx.register(Box::new(cube_prefilter));
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.intensity);
    qcx.deregister(&mut self.cube_map);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let prefiltered = cx
      .take_result(self.cube_map)
      .unwrap()
      .downcast::<LockReadGuardHolder<
        FastHashMap<EntityHandle<SceneTextureCubeEntity>, PreFilterMapGenerationResult>,
      >>()
      .unwrap();

    let intensity = cx.take_multi_updater_updated(self.intensity).unwrap();

    Box::new(IBLLightingComponentProvider {
      prefiltered: *prefiltered,
      brdf_lut: self.brdf_lut.clone(),
      uniform: intensity,
      access: global_database().read_foreign_key::<SceneHDRxEnvBackgroundCubeMap>(),
    })
  }
}

struct IBLLightingComponentProvider {
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
  map: Arc<RwLock<FastHashMap<K, PreFilterMapGenerationResult>>>,
  gpu: GPU,
}

impl<K: CKey> ReactiveGeneralQuery for CubeMapWithPrefilter<K> {
  type Output = Box<dyn Any>;

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

    Box::new(self.map.make_read_holder())
  }
}
