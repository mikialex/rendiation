use std::num::NonZeroU32;

use rendiation_area_lighting::AreaLightUniformLightList;
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

pub fn use_lighting(qcx: &mut impl QueryGPUHookCx, ndc: ViewerNDC) {
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

  let dir_lights = use_directional_light_uniform(qcx, &config, ndc.clone());
  let spot_lights = use_scene_spot_light_uniform(qcx, &config, ndc.clone());
  let point_lights = use_scene_point_light_uniform(qcx);

  let scene_ids = use_scene_id_provider(qcx);
}

pub struct LightSystem {
  reversed_depth: bool,
  memory: FunctionMemory,
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  pub tonemap: ToneMap,
  material_defer_lighting_supports: DeferLightingMaterialRegistry,
  pub opaque_scene_content_lighting_technique: LightingTechniqueKind,
}

impl LightSystem {
  pub fn new_and_register(qcx: &mut ReactiveQueryCtx, gpu: &GPU, reversed_depth: bool) -> Self {
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

    let mut internal = Box::new(
      DifferentLightRenderImplProvider::default()
        .with_light(DirectionalUniformLightList::new(
          qcx,
          directional_uniform_array(gpu),
          directional_light_shadow_address,
          reversed_depth,
        ))
        .with_light(SpotLightUniformLightList::new(
          qcx,
          spot_uniform_array(gpu),
          spot_light_shadow_address,
          reversed_depth,
        ))
        .with_light(PointLightUniformLightList::default())
        .with_light(AreaLightUniformLightList {
          light: Default::default(),
          ltc_1,
          ltc_2,
        })
        .with_light(IBLProvider::new(gpu)),
    );

    internal.register(qcx, gpu);
    let mut scene_ids = SceneIdProvider::default();
    scene_ids.register(qcx, gpu);

    Self {
      directional_light_shadow,
      spot_light_shadow,
      internal,
      enable_channel_debugger: false,
      scene_ids,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      tonemap: ToneMap::new(gpu),
      reversed_depth,
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

  pub fn deregister_resource(&mut self, qcx: &mut ReactiveQueryCtx) {
    self.internal.deregister(qcx);
    self.scene_ids.deregister(qcx);
  }

  pub fn prepare_and_create_impl(
    &mut self,
    rcx: &mut QueryResultCtx,
    frame_ctx: &mut FrameCtx,
    cx: &mut Context,
    renderer: &dyn SceneRenderer<ContentKey = SceneContentKey>,
    target_scene: EntityHandle<SceneEntity>,
  ) -> LightingRenderingCx {
    self.tonemap.update(frame_ctx.gpu);

    let key = SceneContentKey {
      only_alpha_blend_objects: None,
    };

    // we could just use empty pass dispatcher, because the color channel not exist at all
    let depth = ();

    let content = |proj: Mat4<f32>, world: Mat4<f32>, frame_ctx: &mut FrameCtx| {
      let camera = UniformBufferDataView::create(
        &frame_ctx.gpu.device,
        CameraGPUTransform::from(CameraTransform::new(proj, world)),
      );
      let camera = Box::new(CameraGPU { ubo: camera }) as Box<dyn RenderComponent>;

      // todo custom cow
      renderer.extract_and_make_pass_content(key, target_scene, todo!(), frame_ctx, &depth)
    };

    let ds = self.directional_light_shadow.update_shadow_maps(
      cx,
      frame_ctx,
      &content,
      self.reversed_depth,
    );

    let ss =
      self
        .spot_light_shadow
        .update_shadow_maps(cx, frame_ctx, &content, self.reversed_depth);

    let sys = SceneLightSystem {
      scene_ids: self.scene_ids.create_impl(rcx),
      system: self,
      imp: self.internal.create_impl(rcx),
    };

    LightingRenderingCx {
      lighting: sys,
      tonemap: &self.tonemap,
      deferred_mat_supports: &self.material_defer_lighting_supports,
      lighting_method: self.opaque_scene_content_lighting_technique,
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
pub struct DefaultDisplayWriter;
impl ShaderHashProvider for DefaultDisplayWriter {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for DefaultDisplayWriter {}
impl GraphicsShaderProvider for DefaultDisplayWriter {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let default = builder.query_or_insert_default::<DefaultDisplay>();
      builder.store_fragment_out(0, default)
    })
  }
}
