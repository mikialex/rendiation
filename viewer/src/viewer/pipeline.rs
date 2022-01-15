use webgpu::*;

use crate::*;

pub struct ViewerPipeline {
  forward: ForwardScene,
  highlight: HighLighter,
  background: BackGroundRendering,
}

impl ViewerPipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      forward: Default::default(),
      highlight: HighLighter::new(gpu),
      background: Default::default(),
    }
  }
}

impl ViewerPipeline {
  #[rustfmt::skip]
  pub fn render(&mut self, engine: &RenderEngine, content: &mut Viewer3dContent) {
    let scene = &mut content.scene;

    let mut scene_depth = depth_attachment().request(engine);

    let mut msaa_color = engine.multisampled_attachment().request(engine);
    let mut msaa_depth = engine.multisampled_depth_attachment().request(engine);

    let mut widgets_result = attachment().request(engine);

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render_by(&mut content.axis_helper)
      .render_by(&mut content.grid_helper)
      .render_by(&mut content.camera_helpers)
      .run(engine, scene);

    let mut final_compose = pass("compose-all")
      .with_color(engine.screen(), scene.get_main_pass_load_op())
      .with_depth(scene_depth.write(), clear(1.));

    final_compose
      .render(&mut self.background)
      .render(&mut self.forward);

    let mut highlight_compose = (!content.selections.is_empty()).then(||{
       let mut selected = attachment()
        .format(webgpu::TextureFormat::Rgba8Unorm)
        .request(engine);

      pass("highlight-selected-mask")
        .with_color(selected.write(), clear(color_same(0.)))
        .render_by(&mut highlight(&content.selections))
        .run(engine, scene);

      self.highlight.draw(selected.read_into())
    });

    let mut copy_frame = copy_frame(widgets_result.read_into());

    final_compose
      .render(&mut highlight_compose)
      .render(&mut copy_frame);

    final_compose.run(engine, scene);

  }
}
