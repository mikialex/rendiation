use webgpu::*;

use crate::*;

pub struct ViewerPipeline {
  highlight: HighLighter,
  blur: CrossBlurData,
}

impl ViewerPipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
      blur: CrossBlurData::new(gpu),
    }
  }
}

impl ViewerPipeline {
  #[rustfmt::skip]
  pub fn render(
    &mut self,
    ctx: &mut FrameCtx,
    content: &mut Viewer3dContent,
    final_target: RenderTargetView,
  ) {
    let scene = &mut content.scene;

    let mut scene_depth = depth_attachment().request(ctx);

    let mut msaa_color = ctx.multisampled_attachment().request(ctx);
    let mut msaa_depth = ctx.multisampled_depth_attachment().request(ctx);

    let mut widgets_result = attachment().request(ctx);

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render(ctx)
      .by(scene.by_main_camera(&mut content.axis_helper))
      .by(scene.by_main_camera(&mut content.grid_helper))
      .by(scene.by_main_camera(&mut content.gizmo))
      .by(scene.by_main_camera_and_self(&mut content.camera_helpers));

    let highlight_compose = (!content.selections.is_empty())
    .then(|| self.highlight.draw(&content.selections, ctx, scene));

    let mut scene_result = attachment().request(ctx);

    pass("scene")
      .with_color(scene_result.write(), get_main_pass_load_op(scene))
      .with_depth(scene_depth.write(), clear(1.))
      .render(ctx)
      .by(scene.by_main_camera_and_self(BackGroundRendering))
      .by(scene.by_main_camera_and_self(ForwardScene))
      .by(copy_frame(widgets_result.read_into(), BlendState::PREMULTIPLIED_ALPHA_BLENDING.into()));

    

    // let scene_result = draw_cross_blur(&self.blur, scene_result.read_into(), ctx);


    pass("compose-all")
      .with_color(final_target, load())
      .with_depth(scene_depth.write(), clear(1.))
      .render(ctx)
      .by(copy_frame(scene_result.read_into(), None))
      .by(highlight_compose);
      // .by(copy_frame(widgets_result.read_into(), BlendState::PREMULTIPLIED_ALPHA_BLENDING.into()));
  }
}
