use std::num::NonZeroU32;

use rendiation_area_lighting::{area_light_uniform_array, SceneAreaLightingProvider};
use rendiation_lighting_shadow_map::*;
use rendiation_texture_gpu_base::create_gpu_texture2d;
use rendiation_texture_gpu_process::{ToneMap, ToneMapType};

mod debug_channels;
mod ibl;
mod light_pass;
mod punctual;
mod shadow;

use debug_channels::*;
use ibl::*;
pub use light_pass::*;
use punctual::*;
use rendiation_webgpu_reactive_utils::*;
pub use shadow::*;

use crate::*;

pub fn use_lighting(
  qcx: &mut impl QueryGPUHookCx,
  ndc: ViewerNDC,
) -> Option<LightingRenderingCxPrepareCtx> {
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

  let dir_lights = use_directional_light_uniform(qcx, &config, ndc);
  let spot_lights = use_scene_spot_light_uniform(qcx, &config, ndc);
  let point_lights = use_scene_point_light_uniform(qcx);
  let area_lights = use_area_light_uniform(qcx);
  let ibl = use_ibl(qcx);

  let scene_ids = use_scene_id_provider(qcx);

  qcx.when_render(|| LightingRenderingCxPrepareCtx {
    dir_lights: dir_lights.unwrap(),
    spot_lights: spot_lights.unwrap(),
    point_lights: point_lights.unwrap(),
    area_lights: area_lights.unwrap(),
    ibl: ibl.unwrap(),
    scene_ids: scene_ids.unwrap(),
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
    extractor: &DefaultSceneBatchExtractor,
    target_scene: EntityHandle<SceneEntity>,
  ) -> LightingRenderingCx {
    self.tonemap.update(frame_ctx.gpu);

    let key = SceneContentKey {
      only_alpha_blend_objects: None,
    };

    let content =
      |proj: Mat4<f32>, world: Mat4<f64>, frame_ctx: &mut FrameCtx, desc: ShadowPassDesc| {
        let camera = UniformBufferDataView::create(
          &frame_ctx.gpu.device,
          CameraGPUTransform::from(CameraTransform::new(proj, world)),
        );

        // we could just use empty pass dispatcher, because the color channel not exist at all
        let depth = ();
        let camera = Box::new(CameraGPU { ubo: camera }) as Box<dyn RenderComponent>;
        let batch = extractor.extract_scene_batch(target_scene, key, frame_ctx);
        let mut content = renderer.make_scene_batch_pass_content(batch, &camera, &depth, frame_ctx);

        desc.render_ctx(frame_ctx).by(&mut content);
      };

    let ds = instance
      .dir_lights
      .update_shadow_maps(frame_ctx, &content, reversed_depth);

    let ss = instance
      .spot_lights
      .update_shadow_maps(frame_ctx, &content, reversed_depth);

    let imp = Box::new(LightingComputeComponentGroupProvider {
      lights: vec![
        Box::new(ds),
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
}

impl LightSystem {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      enable_channel_debugger: false,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      tonemap: ToneMap::new(gpu),
      material_defer_lighting_supports: DeferLightingMaterialRegistry::default()
        .register_material_impl::<PbrSurfaceEncodeDecode>(),
      opaque_scene_content_lighting_technique: LightingTechniqueKind::Forward,
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui, is_hdr_rendering: bool) {
    ui.checkbox(&mut self.enable_channel_debugger, "enable channel debug");

    if is_hdr_rendering {
      ui.label("tonemap is disabled when hdr display enabled");
      self.tonemap.ty = ToneMapType::None;
    } else {
      if self.tonemap.ty == ToneMapType::None {
        self.tonemap.ty = ToneMapType::ACESFilmic;
      }
      egui::ComboBox::from_label("Tone mapping type")
        .selected_text(format!("{:?}", &self.tonemap.ty))
        .show_ui(ui, |ui| {
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
  ) -> Box<dyn RenderComponent + '_> {
    self.get_scene_lighting_component(
      scene,
      Box::new(DirectGeometryProvider),
      Box::new(LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider(PhysicalShading)),
    )
  }

  pub fn get_scene_lighting_component<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
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

    let scene_id = self.scene_ids.get(&scene).unwrap().clone();

    light
      .push(&system.tonemap as &dyn RenderComponent) //
      .push(LightingComputeComponentAsRenderComponent {
        scene_id,
        geometry_constructor,
        surface_constructor,
        lighting: self.imp.get_scene_lighting(scene).unwrap(),
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
      let l = builder.query::<LDRLightResult>();
      let alpha = builder.try_query::<AlphaChannel>().unwrap_or(val(1.0));
      builder.register::<DefaultDisplay>((l, alpha));
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
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let default = builder.query_or_insert_default::<DefaultDisplay>();
      builder.store_fragment_out(self.write_channel_index, default)
    })
  }
}

fn use_area_light_uniform(qcx: &mut impl QueryGPUHookCx) -> Option<SceneAreaLightingProvider> {
  let uniform = qcx.use_uniform_array_buffers(area_light_uniform_array);

  let (qcx, lut) = qcx.use_gpu_init(|gpu| {
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

  qcx.when_render(|| -> _ {
    SceneAreaLightingProvider {
      ltc_1: lut.0.clone(),
      ltc_2: lut.1.clone(),
      uniform: uniform.unwrap(),
    }
  })
}
