use std::sync::Arc;

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_lighting_ibl::{
  generate_pre_filter_map, IBLLightingComponent, IblShaderInfo, PreFilterMapGenerationConfig,
  PreFilterMapGenerationResult,
};
use rendiation_texture_gpu_base::create_gpu_texture2d;
use rendiation_texture_gpu_process::ToneMap;
use rendiation_webgpu_reactive_utils::*;

use super::ScreenChannelDebugger;
use crate::*;

pub struct LightSystem {
  internal: Box<dyn RenderImplProvider<Box<dyn LightingComputeComponent>>>,
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  tonemap: ToneMap,
}

// pub trait LightSystemSceneProvider {
//   fn get_scene_lighting(
//     &self,
//     scene: EntityHandle<SceneEntity>,
//   ) -> Box<dyn LightingComputeComponent>;
// }

// struct LightSystemSceneProviderDefault {}

impl LightSystem {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      internal: Box::new(
        DifferentLightRenderImplProvider::default()
          .with_light(DirectionalUniformLightList::default())
          .with_light(SpotLightUniformLightList::default())
          .with_light(PointLightUniformLightList::default())
          .with_light(IBLProvider::new(gpu)),
      ),
      enable_channel_debugger: false,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      tonemap: ToneMap::new(gpu),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    ui.checkbox(&mut self.enable_channel_debugger, "enable channel debug");
    self.tonemap.mutate_exposure(|e| {
      ui.add(
        egui::Slider::new(e, 0.0..=2.0)
          .step_by(0.05)
          .text("exposure"),
      );
    });
  }

  pub fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.internal.register_resource(source, cx);
  }

  pub fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
    frame_ctx: &mut FrameCtx,
  ) -> Box<dyn RenderComponent + '_> {
    let mut light = RenderVec::default();

    if self.enable_channel_debugger {
      light.push(&self.channel_debugger as &dyn RenderComponent);
    } else {
      light.push(LDROutput);
    }

    self.tonemap.update(frame_ctx.gpu);

    light
      .push(&self.tonemap as &dyn RenderComponent) //
      .push(LightingComputeComponentAsRenderComponent(
        self.internal.create_impl(res),
      ));

    Box::new(light)
  }
}

struct LDROutput;

impl ShaderHashProvider for LDROutput {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for LDROutput {}
impl GraphicsShaderProvider for LDROutput {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let l = builder.query::<LDRLightResult>();
      builder.register::<DefaultDisplay>((l, val(1.0)));
    })
  }
}

struct IBLProvider {
  brdf_lut: GPU2DTextureView,
  intensity: UpdateResultToken,
  // todo
  // note, currently the cube map is standalone maintained, this is wasteful if user shared it elsewhere
  cube_map: UpdateResultToken,
}

impl IBLProvider {
  pub fn new(cx: &GPU) -> Self {
    let brdf_lut_bitmap_png = include_bytes!("./brdf_lut.png");
    let png_decoder = png::Decoder::new(brdf_lut_bitmap_png.as_slice());
    let mut png_reader = png_decoder.read_info().unwrap();
    let mut buf = vec![0; png_reader.output_buffer_size()];
    png_reader.next_frame(&mut buf).unwrap();

    let (width, height) = png_reader.info().size();
    let brdf_lut = create_gpu_texture2d(
      cx,
      &GPUBufferImage {
        data: buf,
        format: TextureFormat::Rgba8Uint, // lut is linear
        size: Size::from_u32_pair_min_one((width, height)),
      },
    );

    Self {
      brdf_lut,
      intensity: Default::default(),
      cube_map: Default::default(),
    }
  }
}

impl RenderImplProvider<Box<dyn LightingComputeComponent>> for IBLProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
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

    self.intensity = source.register_multi_updater(intensity);

    let cube_prefilter = CubeMapWithPrefilter {
      inner: RwLock::new(gpu_texture_cubes(cx, Default::default())),
      map: Default::default(),
      gpu: cx.clone(),
    };

    self.cube_map = source.register(Box::new(cube_prefilter));
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.intensity);
    source.deregister(&mut self.cube_map);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn LightingComputeComponent> {
    let prefiltered = res
      .take_result(self.cube_map)
      .unwrap()
      .downcast::<LockReadGuardHolder<
        FastHashMap<EntityHandle<SceneTextureCubeEntity>, PreFilterMapGenerationResult>,
      >>()
      .unwrap();

    // let intensity = res.take_multi_updater_updated(self.intensity).unwrap();

    Box::new(IBLLightingComponent {
      prefiltered: todo!(),
      brdf_lut: self.brdf_lut.clone(),
      uniform: todo!(),
    })
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
