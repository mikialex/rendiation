use webgpu::*;

use crate::*;

pub struct ViewerPipeline {
  highlight: HighLighter,
}

impl ViewerPipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      highlight: HighLighter::new(gpu),
    }
  }
}

impl ViewerPipeline {
  #[rustfmt::skip]
  pub fn render(&mut self, ctx: &mut FrameCtx, content: &mut Viewer3dContent) {
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
      .by(&mut scene.by_main_camera(content.axis_helper))
      .by(&mut scene.by_main_camera(content.grid_helper))
      .by(&mut scene.by_main_camera_and_self(content.camera_helpers));

    let mut highlight_compose = (!content.selections.is_empty()).then(|| {
      let mut selected = attachment()
        .format(webgpu::TextureFormat::Rgba8Unorm)
        .request(ctx);

      pass("highlight-selected-mask")
        .with_color(selected.write(), clear(color_same(0.)))
        .render(ctx)
        .by(&mut scene.by_main_camera(highlight(&content.selections)));

      self.highlight.draw(selected.read_into())
    });

    let mut final_compose = pass("compose-all")
      .with_color(ctx.screen(), scene.get_main_pass_load_op())
      .with_depth(scene_depth.write(), clear(1.))
      .render(ctx)
      .by(&mut scene.by_main_camera_and_self(BackGroundRendering))
      .by(&mut scene.by_main_camera_and_self(ForwardScene))
      .by(&mut highlight_compose)
      .by(&mut copy_frame(widgets_result.read_into()));
  }
}
