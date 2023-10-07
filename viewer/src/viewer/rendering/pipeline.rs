use std::task::Context;

use webgpu::*;

use crate::*;

pub struct ViewerPipeline {
  highlight: HighLighter,
  taa: TAA,
  enable_ssao: bool,
  ssao: SSAO,
  _blur: CrossBlurData,
  enable_channel_debugger: bool,
  channel_debugger: ScreenChannelDebugger,
  tonemap: ToneMap,
}

impl ViewerPipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      _blur: CrossBlurData::new(gpu),
      taa: TAA::new(gpu),
      enable_ssao: true,
      ssao: SSAO::new(gpu),
      enable_channel_debugger: false,
      channel_debugger: ScreenChannelDebugger::default_useful(),
      tonemap: ToneMap::new(gpu),
    }
  }

  /// some effect maybe take continuously draw in next frames to finish
  pub fn setup_render_waker(&self, _cx: &mut Context) {
    // todo
  }
}

impl ViewerPipeline {
  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    content: &Viewer3dContent,
    final_target: &RenderTargetView,
    scene: &SceneRenderResourceGroup,
  ) {
    let mut widgets = content.widgets.borrow_mut();

    let mut mip_gen = scene.resources.bindable_ctx.gpu.mipmap_gen.borrow_mut();
    mip_gen.flush_mipmap_gen_request(ctx);
    let mut single_proj_sys = scene
      .scene_resources
      .shadows
      .single_proj_sys
      .write()
      .unwrap();
    single_proj_sys.update_depth_maps(ctx, scene);
    drop(single_proj_sys);

    let mut msaa_color = attachment().sample_count(4).request(ctx);
    let mut msaa_depth = depth_attachment().sample_count(4).request(ctx);
    let mut widgets_result = attachment().request(ctx);

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render_ctx(ctx)
      .by(scene.by_main_camera_and_self(&mut widgets.axis_helper))
      .by(scene.by_main_camera_and_self(&mut widgets.grid_helper))
      .by(scene.by_main_camera_and_self(&mut widgets.gizmo))
      .by(scene.by_main_camera_and_self(&mut widgets.camera_helpers));

    let highlight_compose = (!content.selections.is_empty()).then(|| {
      self
        .highlight
        .draw(content.selections.iter_selected().cloned(), ctx, scene)
    });

    let taa_result =
      self
        .taa
        .render_aa_content(scene.scene.get_active_camera(), scene, ctx, |ctx| {
          let mut scene_result = attachment().request(ctx);
          let mut scene_depth = depth_attachment().request(ctx);

          let mut cameras = scene.scene_resources.cameras.write().unwrap();
          let camera_gpu = cameras
            .get_camera_gpu_mut(scene.scene.get_active_camera())
            .unwrap();

          let ao = self.enable_ssao.then(|| {
            let ao = self.ssao.draw(ctx, &scene_depth, camera_gpu);
            copy_frame(
              ao.read_into(),
              BlendState {
                color: BlendComponent {
                  src_factor: BlendFactor::Dst,
                  dst_factor: BlendFactor::One,
                  operation: BlendOperation::Add,
                },
                alpha: BlendComponent::REPLACE,
              }
              .into(),
            )
          });
          drop(cameras);

          // these pass will get correct gpu camera?
          pass("scene")
            .with_color(scene_result.write(), get_main_pass_load_op(scene.scene))
            .with_depth(scene_depth.write(), clear(1.))
            .render_ctx(ctx)
            .by(scene.by_main_camera_and_self(BackGroundRendering))
            .by(
              scene.by_main_camera_and_self(ForwardScene {
                tonemap: &self.tonemap,
                debugger: self
                  .enable_channel_debugger
                  .then_some(&self.channel_debugger),
              }),
            )
            .by(scene.by_main_camera_and_self(&mut widgets.ground)) // transparent, should go after opaque
            .by(ao);

          NewTAAFrameSample {
            new_color: scene_result,
            new_depth: scene_depth,
          }
        });

    let main_scene_content = copy_frame(taa_result.read(), None);

    let scene_msaa_widgets = copy_frame(
      widgets_result.read_into(),
      BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
    );

    pass("compose-all")
      .with_color(final_target.clone(), load())
      .render_ctx(ctx)
      .by(main_scene_content)
      .by(highlight_compose)
      .by(scene_msaa_widgets);
  }
}
