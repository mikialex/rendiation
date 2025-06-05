use rendiation_texture_gpu_process::HighLightMaskDispatcher;

use crate::*;

pub fn use_high_lighter(
  cx: &mut Viewer3dRenderingCx,
  renderer: Option<&dyn SceneRenderer>,
) -> Option<Box<dyn PassContent>> {
  let (cx, highlight) = cx.use_plain_state::<HighLighter>();

  cx.on_render(|frame_ctx, content| {
    let mut highlight_compose = (content.selected_target.is_some()).then(|| {
      let masked_content = renderer.render_models(
        Box::new(IteratorAsHostRenderBatch(content.selected_target)),
        CameraRenderSource::Scene(content.main_camera),
        &HighLightMaskDispatcher,
        frame_ctx,
      );
      highlight.draw(frame_ctx, masked_content)
    });
  });
}
