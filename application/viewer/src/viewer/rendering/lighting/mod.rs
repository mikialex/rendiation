use std::num::NonZeroU32;

use rendiation_area_lighting::{use_area_light_uniform_array, SceneAreaLightingProvider};
use rendiation_lighting_shadow_map::*;
use rendiation_texture_gpu_base::create_gpu_texture2d;
use rendiation_texture_gpu_process::{ToneMap, ToneMapType};

mod debug_channels;
mod ibl;
mod light_pass;
mod punctual;
mod shadow;
mod shadow_cascade;

use debug_channels::*;
use ibl::*;
pub use light_pass::*;
use punctual::*;
use rendiation_webgpu_hook_utils::*;
pub use shadow::*;
pub use shadow_cascade::*;

use crate::*;

pub fn use_lighting(
  cx: &mut QueryGPUHookCx,
  lighting_sys: &LightSystem,
  ndc: ViewerNDC,
  viewports: &[ViewerViewPort],
) -> Option<LightingRenderingCxPrepareCtx> {
  let size = Size::from_u32_pair_min_one((2048, 2048));
  let config = MultiLayerTexturePackerConfig {
    max_size: SizeWithDepth {
      depth: NonZeroU32::new(3).unwrap(),
      size,
    },
    init_size: SizeWithDepth {
      depth: NonZeroU32::new(2).unwrap(), // start with 2 layers to support webgl
      size,
    },
  };

  let dir_lights = use_directional_light_uniform(cx, &config, viewports, lighting_sys, ndc);
  let spot_lights = use_scene_spot_light_uniform(cx, &config, lighting_sys, ndc);
  let point_lights = use_scene_point_light_uniform(cx);
  let area_lights = use_area_light_uniform(cx);
  let ibl = use_ibl(cx);

  let scene_ids = use_scene_id_provider(cx);

  cx.when_render(|| LightingRenderingCxPrepareCtx {
    dir_lights: dir_lights.unwrap(),
    spot_lights: spot_lights.unwrap(),
    point_lights: point_lights.unwrap(),
    area_lights: area_lights.unwrap(),
    ibl: ibl.unwrap(),
    scene_ids,
  })
}

pub struct LightingRenderingCxPrepareCtx {
  dir_lights: SceneDirectionalLightingPreparer,
  spot_lights: SceneSpotLightingPreparer,
  point_lights: ScenePointLightingProvider,
  area_lights: SceneAreaLightingProvider,
  ibl: IBLLightingComponentProvider,
  scene_ids: SceneIdUniformBufferAccess,
}

impl LightSystem {
  pub fn prepare(
    &self,
    instance: LightingRenderingCxPrepareCtx,
    frame_ctx: &mut FrameCtx,
    reversed_depth: bool,
    renderer: &dyn SceneRenderer,
    extractor: &ViewerBatchExtractor,
    target_scene: EntityHandle<SceneEntity>,
  ) -> LightingRenderingCx<'_> {
    self.tonemap.update(frame_ctx.gpu);

    let key = SceneContentKey {
      only_alpha_blend_objects: None,
    };

    // this a bit hacky, but it should works
    let mut shadow_id = 0;
    let mut content =
      |proj: Mat4<f32>, world: Mat4<f64>, frame_ctx: &mut FrameCtx, desc: ShadowPassDesc| {
        let camera = UniformBufferDataView::create(
          &frame_ctx.gpu.device,
          CameraGPUTransform::from(CameraTransform::new(proj, world)),
        );

        // we could just use empty pass dispatcher, because the color channel not exist at all
        let depth = ();
        let camera = Box::new(CameraGPU { ubo: camera }) as Box<dyn RenderComponent>;
        let batch = extractor.extract_scene_batch(target_scene, key, renderer);

        frame_ctx.keyed_scope(&shadow_id, |frame_ctx| {
          let mut content =
            renderer.make_scene_batch_pass_content(batch, &camera, &depth, frame_ctx);

          desc.render_ctx(frame_ctx).by(&mut content);
        });
        shadow_id += 1;
      };

    let ds = instance
      .dir_lights
      .update_shadow_maps(frame_ctx, &mut content, reversed_depth);

    let ss = instance
      .spot_lights
      .update_shadow_maps(frame_ctx, &mut content, reversed_depth);

    let imp = Box::new(LightingComputeComponentGroupProvider {
      lights: vec![
        ds,
        Box::new(ss),
        Box::new(instance.point_lights),
        Box::new(instance.area_lights),
        Box::new(instance.ibl),
      ],
    });

    let sys = SceneLightSystem {
      scene_ids: instance.scene_ids,
      system: self,
      imp,
    };

    LightingRenderingCx {
      lighting: sys,
      tonemap: &self.tonemap,
      deferred_mat_supports: &self.material_defer_lighting_supports,
      lighting_method: self.opaque_scene_content_lighting_technique,
    }
  }
}

pub struct LightSystem {
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  pub tonemap: ToneMap,
  material_defer_lighting_supports: DeferLightingMaterialRegistry,
  pub opaque_scene_content_lighting_technique: LightingTechniqueKind,
  pub enable_shadow: bool,
  pub use_cascade_shadowmap_for_directional_lights: bool,
  pub cascade_shadow_split_linear_log_blend_ratio: f32,
}

impl LightSystem {
  pub fn new(gpu: &GPU, init_config: &ViewerInitConfig) -> Self {
    Self {
      enable_shadow: init_config.enable_shadow,
      enable_channel_debugger: false,
      cascade_shadow_split_linear_log_blend_ratio: 0.95,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      use_cascade_shadowmap_for_directional_lights: false,
      tonemap: ToneMap::new(gpu),
      material_defer_lighting_supports: DeferLightingMaterialRegistry::default()
        .register_material_impl::<PbrSurfaceEncodeDecode>(),
      opaque_scene_content_lighting_technique: LightingTechniqueKind::Forward,
    }
  }

  pub fn egui(&mut self, ui: &mut UiWithChangeInfo, is_hdr_rendering: bool) {
    ui.checkbox(&mut self.enable_channel_debugger, "enable channel debug");
    ui.checkbox(
      &mut self.use_cascade_shadowmap_for_directional_lights,
      "use cascade shadowmap for directional lights",
    );

    if self.use_cascade_shadowmap_for_directional_lights {
      ui.add(
        egui::Slider::new(
          &mut self.cascade_shadow_split_linear_log_blend_ratio,
          0.0..=1.0,
        )
        .step_by(0.01)
        .text("split_linear_log_blend_ratio"),
      );
    }

    if is_hdr_rendering {
      ui.label("tonemap is disabled when hdr display enabled");
      self.tonemap.ty = ToneMapType::None;
    } else {
      if self.tonemap.ty == ToneMapType::None {
        self.tonemap.ty = ToneMapType::ACESFilmic;
      }
      egui::ComboBox::from_label("Tone mapping type")
        .selected_text(format!("{:?}", &self.tonemap.ty))
        .show_ui_changed(ui, |ui| {
          ui.selectable_value(&mut self.tonemap.ty, ToneMapType::Linear, "Linear");
          ui.selectable_value(&mut self.tonemap.ty, ToneMapType::Cineon, "Cineon");
          ui.selectable_value(&mut self.tonemap.ty, ToneMapType::Reinhard, "Reinhard");
          ui.selectable_value(&mut self.tonemap.ty, ToneMapType::ACESFilmic, "ACESFilmic");
        });

      self.tonemap.mutate_exposure(|e| {
        ui.add(
          egui::Slider::new(e, 0.0..=2.0)
            .step_by(0.05)
            .text("exposure"),
        );
      });
    }
  }
}

pub struct SceneLightSystem<'a> {
  system: &'a LightSystem,
  scene_ids: SceneIdUniformBufferAccess,
  imp: Box<dyn LightSystemSceneProvider>,
}

impl SceneLightSystem<'_> {
  pub fn get_scene_forward_lighting_component(
    &self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Box<dyn RenderComponent + '_> {
    self.get_scene_lighting_component(
      scene,
      camera,
      Box::new(DirectGeometryProvider),
      Box::new(LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider(PhysicalShading)),
    )
  }

  pub fn get_scene_lighting_component<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    geometry_constructor: Box<dyn GeometryCtxProvider + 'a>,
    surface_constructor: Box<dyn LightableSurfaceProvider + 'a>,
  ) -> Box<dyn RenderComponent + 'a> {
    let mut light = RenderVec::default();

    let system = &self.system;

    if system.enable_channel_debugger {
      light.push(&system.channel_debugger as &dyn RenderComponent);
    } else {
      light.push(LDROutput);
    }

    let scene_id = self.scene_ids.get(&scene.into_raw()).unwrap().clone();

    light
      .push(&system.tonemap as &dyn RenderComponent) //
      .push(LightingComputeComponentAsRenderComponent {
        scene_id,
        geometry_constructor,
        surface_constructor,
        lighting: self.imp.get_scene_lighting(scene, camera).unwrap(),
      });

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
      if builder.contains_type_tag::<LightableSurfaceTag>() {
        let l = builder.query::<LDRLightResult>();
        let alpha = builder.try_query::<AlphaChannel>().unwrap_or(val(1.0));
        builder.register::<DefaultDisplay>((l, alpha));
      }
    })
  }
}

/// we disable the base dispatch auto write in scene pass content. however in some times
/// we still need to write to the default display, use this as the outer dispatcher to inject write logic
pub struct DefaultDisplayWriter {
  pub write_channel_index: usize,
}

impl DefaultDisplayWriter {
  pub fn extend_pass_desc(
    pass: &mut RenderPassDescription,
    target: &RenderTargetView,
    op: impl Into<Operations<rendiation_webgpu::Color>>,
  ) -> Self {
    Self {
      write_channel_index: pass.push_color(target, op),
    }
  }
}

impl ShaderHashProvider for DefaultDisplayWriter {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.write_channel_index.hash(hasher);
  }
}
impl ShaderPassBuilder for DefaultDisplayWriter {}
impl GraphicsShaderProvider for DefaultDisplayWriter {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    if ENABLE_DEFAULT_DISPLAY_DEBUG {
      builder.fragment(|_, _| {
        DEFAULT_DISPLAY_DEBUG
          .with_borrow_mut(|v| *v = Some(zeroed_val::<Vec3<f32>>().make_local_var()));
      })
    }
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let debug = DEFAULT_DISPLAY_DEBUG.with_borrow_mut(|v| v.take());

      if let Some(debug) = debug {
        builder.store_fragment_out(self.write_channel_index, vec4_node((debug.load(), val(1.))));
      } else {
        let default = builder.query_or_insert_default::<DefaultDisplay>();
        builder.store_fragment_out(self.write_channel_index, default)
      }
    })
  }
}

fn use_area_light_uniform(cx: &mut QueryGPUHookCx) -> Option<SceneAreaLightingProvider> {
  let uniform = use_area_light_uniform_array(cx);

  let (cx, lut) = cx.use_gpu_init(|gpu, _| {
    let ltc_1 = include_bytes!("./ltc_1.bin");
    let ltc_1 = create_gpu_texture2d(
      gpu,
      &GPUBufferImage {
        data: ltc_1.as_slice().to_vec(),
        format: TextureFormat::Rgba16Float,
        size: Size::from_u32_pair_min_one((64, 64)),
      },
    );
    let ltc_2 = include_bytes!("./ltc_2.bin");
    let ltc_2 = create_gpu_texture2d(
      gpu,
      &GPUBufferImage {
        data: ltc_2.as_slice().to_vec(),
        format: TextureFormat::Rgba16Float,
        size: Size::from_u32_pair_min_one((64, 64)),
      },
    );
    (ltc_1, ltc_2)
  });

  cx.when_render(|| -> _ {
    SceneAreaLightingProvider {
      ltc_1: lut.0.clone(),
      ltc_2: lut.1.clone(),
      uniform,
    }
  })
}
