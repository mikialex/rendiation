use rendiation_texture_gpu_process::ToneMap;

use super::ScreenChannelDebugger;
use crate::*;

pub struct LightSystem {
  internal: Box<dyn RenderImplProvider<Box<dyn LightingComputeComponent>>>,
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  tonemap: ToneMap,
}

impl LightSystem {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      internal: Box::new(
        LightArrayRenderImplProvider::default()
          .with_light(DirectionalUniformLightList::default())
          .with_light(SpotLightUniformLightList::default())
          .with_light(PointLightUniformLightList::default()),
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
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      let l = builder.query::<LDRLightResult>()?;
      builder.register::<DefaultDisplay>((l, val(1.0)));
      Ok(())
    })
  }
}
