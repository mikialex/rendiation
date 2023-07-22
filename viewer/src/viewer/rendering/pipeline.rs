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

    let mut scene_depth = depth_attachment().request(ctx);

    let mut msaa_color = ctx.multisampled_attachment().request(ctx);
    let mut msaa_depth = ctx.multisampled_depth_attachment().request(ctx);

    let mut widgets_result = attachment().request(ctx);

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render(ctx)
      .by(scene.by_main_camera_and_self(&mut widgets.axis_helper))
      .by(scene.by_main_camera_and_self(&mut widgets.grid_helper))
      .by(scene.by_main_camera_and_self(&mut widgets.gizmo))
      .by(scene.by_main_camera_and_self(&mut widgets.camera_helpers));

    let highlight_compose = (!content.selections.is_empty()).then(|| {
      self
        .highlight
        .draw(content.selections.as_renderables(), ctx, scene)
    });

    let mut scene_result = attachment().request(ctx);

    {
      let jitter = self.taa.next_jitter();
      let mut cameras = scene.scene_resources.cameras.write().unwrap();
      let gpu = cameras
        .get_camera_gpu_mut(scene.scene.get_active_camera())
        .unwrap();
      gpu
        .ubo
        .mutate(|uniform| uniform.jitter_normalized = jitter)
        .upload(&ctx.gpu.queue);
      gpu.enable_jitter = true;
    }

    let ao = self.enable_ssao.then(|| {
      let cameras = scene.scene_resources.cameras.read().unwrap();
      let camera_gpu = cameras
        .get_camera_gpu(scene.scene.get_active_camera())
        .unwrap();

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

    pass("scene")
      .with_color(scene_result.write(), get_main_pass_load_op(scene.scene))
      .with_depth(scene_depth.write(), clear(1.))
      .render(ctx)
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

    let mut cameras = scene.scene_resources.cameras.write().unwrap();
    let camera_gpu = cameras
      .get_camera_gpu_mut(scene.scene.get_active_camera())
      .unwrap();
    camera_gpu.enable_jitter = false;

    // let scene_result = draw_cross_blur(&self.blur, scene_result.read_into(), ctx);

    let taa_result = self
      .taa
      .resolve(&scene_result, &scene_depth, ctx, camera_gpu);
    drop(cameras);

    pass("compose-all")
      .with_color(final_target.clone(), load())
      .render(ctx)
      .by(copy_frame(taa_result.read(), None))
      .by(highlight_compose)
      .by(copy_frame(
        widgets_result.read_into(),
        BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
      ));
  }
}
