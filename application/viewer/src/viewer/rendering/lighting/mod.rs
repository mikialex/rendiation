use std::num::NonZeroU32;

use rendiation_lighting_shadow_map::*;
use rendiation_texture_gpu_process::ToneMap;

mod debug_channels;
mod ibl;
mod punctual;
mod shadow;

use debug_channels::*;
use ibl::*;
use punctual::*;
pub use shadow::*;

use crate::*;

pub struct LightSystem {
  internal: Box<dyn RenderImplProvider<Box<dyn LightSystemSceneProvider>>>,
  directional_light_shadow: BasicShadowMapSystem,
  spot_light_shadow: BasicShadowMapSystem,
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  tonemap: ToneMap,
}

impl LightSystem {
  pub fn new_and_register(source: &mut ReactiveQueryJoinUpdater, gpu: &GPU) -> Self {
    let size = Size::from_u32_pair_min_one((2048, 2048));
    let config = MultiLayerTexturePackerConfig {
      max_size: SizeWithDepth {
        depth: NonZeroU32::new(2).unwrap(),
        size,
      },
      init_size: SizeWithDepth {
        depth: NonZeroU32::new(1).unwrap(),
        size,
      },
    };

    let source_proj = global_watch()
      .watch_untyped_key::<DirectionLightShadowBound>()
      .collective_map(|orth| {
        orth
          .unwrap_or(OrthographicProjection {
            left: -20.,
            right: 20.,
            top: 20.,
            bottom: -20.,
            near: 0.,
            far: 1000.,
          })
          .compute_projection_mat::<WebGPUxNDC>()
      })
      .into_boxed();

    let source_world = scene_node_derive_world_mat()
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
      .untyped_entity_handle()
      .into_boxed();

    let (directional_light_shadow, directional_light_shadow_address) = basic_shadow_map_uniform(
      BasicShadowMapSystemInputs {
        source_world,
        source_proj,
        size: global_watch()
          .watch_untyped_key::<BasicShadowMapResolutionOf<DirectionLightBasicShadowInfo>>()
          .collective_map(|size| Size::from_u32_pair_min_one(size.into()))
          .into_boxed(),
        bias: global_watch()
          .watch_untyped_key::<BasicShadowMapBiasOf<DirectionLightBasicShadowInfo>>()
          .into_boxed(),
        enabled: global_watch()
          .watch_untyped_key::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>()
          .into_boxed(),
      },
      config,
      gpu,
    );

    let source_proj = global_watch()
      .watch_untyped_key::<SpotLightHalfConeAngle>()
      .collective_map(|half_cone_angle| {
        PerspectiveProjection {
          near: 0.1,
          far: 2000.,
          fov: Deg::from_rad(half_cone_angle * 2.),
          aspect: 1.,
        }
        .compute_projection_mat::<WebGPUxNDC>()
      })
      .into_boxed();

    let source_world = scene_node_derive_world_mat()
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
      .untyped_entity_handle()
      .into_boxed();

    let (spot_light_shadow, spot_light_shadow_address) = basic_shadow_map_uniform(
      BasicShadowMapSystemInputs {
        source_proj,
        source_world,
        size: global_watch()
          .watch_untyped_key::<BasicShadowMapResolutionOf<SpotLightBasicShadowInfo>>()
          .collective_map(|size| Size::from_u32_pair_min_one(size.into()))
          .into_boxed(),
        bias: global_watch()
          .watch_untyped_key::<BasicShadowMapBiasOf<SpotLightBasicShadowInfo>>()
          .into_boxed(),
        enabled: global_watch()
          .watch_untyped_key::<BasicShadowMapEnabledOf<SpotLightBasicShadowInfo>>()
          .into_boxed(),
      },
      config,
      gpu,
    );

    let mut internal = Box::new(
      DifferentLightRenderImplProvider::default()
        .with_light(DirectionalUniformLightList::new(
          source,
          directional_uniform_array(gpu),
          directional_light_shadow_address,
        ))
        .with_light(SpotLightUniformLightList::new(
          source,
          spot_uniform_array(gpu),
          spot_light_shadow_address,
        ))
        .with_light(PointLightUniformLightList::default())
        .with_light(IBLProvider::new(gpu)),
    );

    internal.register_resource(source, gpu);

    Self {
      directional_light_shadow,
      spot_light_shadow,
      internal,
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

  pub fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.internal.deregister_resource(source);
  }

  pub fn prepare_and_create_impl(
    &mut self,
    res: &mut QueryResultCtx,
    frame_ctx: &mut FrameCtx,
    cx: &mut Context,
    renderer: &dyn SceneRenderer<ContentKey = SceneContentKey>,
    target_scene: EntityHandle<SceneEntity>,
  ) -> SceneLightSystem {
    self.tonemap.update(frame_ctx.gpu);

    let key = SceneContentKey { transparent: false };

    // we could just use empty pass dispatcher, because the color channel not exist at all
    let depth = ();

    let content = |proj: Mat4<f32>, world: Mat4<f32>, frame_ctx: &mut FrameCtx| {
      let camera = UniformBufferDataView::create(
        &frame_ctx.gpu.device,
        CameraGPUTransform::from(CameraTransform::new(proj, world)),
      );
      let camera = Box::new(CameraGPU { ubo: camera });
      let camera = CameraRenderSource::External(camera);

      renderer.extract_and_make_pass_content(key, target_scene, camera, frame_ctx, &depth)
    };

    let ds = self
      .directional_light_shadow
      .update_shadow_maps(cx, frame_ctx, &content);

    let ss = self
      .spot_light_shadow
      .update_shadow_maps(cx, frame_ctx, &content);

    res.type_based_result.register(DirectionalShaderAtlas(ds));
    res.type_based_result.register(SpotShaderAtlas(ss));

    SceneLightSystem {
      system: self,
      imp: self.internal.create_impl(res),
    }
  }
}

pub struct SceneLightSystem<'a> {
  system: &'a LightSystem,
  imp: Box<dyn LightSystemSceneProvider>,
}

impl SceneLightSystem<'_> {
  pub fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Box<dyn RenderComponent + '_> {
    let mut light = RenderVec::default();

    let system = &self.system;

    if system.enable_channel_debugger {
      light.push(&system.channel_debugger as &dyn RenderComponent);
    } else {
      light.push(LDROutput);
    }

    light
      .push(&system.tonemap as &dyn RenderComponent) //
      .push(LightingComputeComponentAsRenderComponent(
        self.imp.get_scene_lighting(scene).unwrap(),
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
