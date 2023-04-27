use webgpu::*;

use crate::*;

pub struct ViewerPipeline {
  highlight: HighLighter,
  taa: TAA,
  enable_ssao: bool,
  ssao: SSAO,
  blur: CrossBlurData,
  forward_lights: ForwardLightingSystem,
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  shadows: ShadowMapSystem,
  tonemap: ToneMap,
}

impl ViewerPipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      blur: CrossBlurData::new(gpu),
      taa: TAA::new(gpu),
      enable_ssao: true,
      ssao: SSAO::new(gpu),
      forward_lights: Default::default(),
      enable_channel_debugger: false,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      shadows: ShadowMapSystem::new(gpu),
      tonemap: ToneMap::new(gpu),
    }
  }
}

impl ViewerPipeline {
  #[rustfmt::skip]
  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    content: &Viewer3dContent,
    final_target: &RenderTargetView,
  ) {
    let scene = &content.scene.read();
    let mut widgets = content.widgets.borrow_mut();

    ctx.resolve_resource_mipmaps();

    LightUpdateCtx {
      forward: &mut self.forward_lights,
      shadows: &mut self.shadows,
      ctx,
      scene,
      node_derives:&content.scene_derived,
    }.update();

    let mut scene_depth = depth_attachment().request(ctx);

    let mut msaa_color = ctx.multisampled_attachment().request(ctx);
    let mut msaa_depth = ctx.multisampled_depth_attachment().request(ctx);

    let mut widgets_result = attachment().request(ctx);

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render(ctx)
      .by(scene.by_main_camera(&mut widgets.axis_helper))
      .by(scene.by_main_camera(&mut widgets.grid_helper))
      .by(scene.by_main_camera(&mut widgets.gizmo))
      .by(scene.by_main_camera_and_self(&mut widgets.camera_helpers));

    let highlight_compose = (!content.selections.is_empty())
    .then(|| self.highlight.draw(&content.selections, ctx, scene.get_active_camera()));

    let mut scene_result = attachment().request(ctx);

    let jitter = self.taa.next_jitter();
    let mut cameras = ctx.scene_resources.cameras.borrow_mut();
    let gpu = cameras.get_with_update(scene.get_active_camera(), &(ctx.gpu, &content.scene_derived));
    gpu.ubo.resource.mutate(|uniform| uniform.set_jitter(jitter)).upload(&ctx.gpu.queue);
    gpu.enable_jitter = true;
    drop(cameras);

    let ao = self.enable_ssao.then(||{
      let ao = self.ssao.draw(ctx, &scene_depth,  scene.get_active_camera());
      copy_frame(ao.read_into(), BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::Dst,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent::REPLACE,
     }.into())
    });

    pass("scene")
      .with_color(scene_result.write(), get_main_pass_load_op(scene))
      .with_depth(scene_depth.write(), clear(1.))
      .render(ctx)
      .by(scene.by_main_camera_and_self(BackGroundRendering))
      .by(scene.by_main_camera_and_self(ForwardScene {
        lights: &self.forward_lights,
        shadow: &self.shadows,
        tonemap: &self.tonemap,
        debugger: self.enable_channel_debugger.then_some(&self.channel_debugger)
      }))
      .by(scene.by_main_camera(&mut widgets.ground)) // transparent, should go after opaque
      .by(ao);

    let mut cameras = ctx.scene_resources.cameras.borrow_mut();
    cameras.get_with_update(scene.get_active_camera(), &(ctx.gpu, &content.scene_derived)).enable_jitter = false;
    drop(cameras);
    // let scene_result = draw_cross_blur(&self.blur, scene_result.read_into(), ctx);

    let taa_result = self.taa.resolve(
      &scene_result,
      &scene_depth,
      ctx,
      scene.get_active_camera()
    );

    pass("compose-all")
      .with_color(final_target.clone(), load())
      .render(ctx)
      .by(copy_frame(taa_result.read(), None))
      .by(highlight_compose)
      .by(copy_frame(widgets_result.read_into(), BlendState::PREMULTIPLIED_ALPHA_BLENDING.into()));
  }
}
