use rendiation_texture_gpu_process::*;

use crate::*;

pub fn use_high_lighter(
  cx: &mut Viewer3dRenderingCx,
  renderer: Option<&dyn SceneRenderer>,
) -> Option<impl PassContent> {
  let (cx, highlight) = cx.use_gpu_state(HighLighter::new);

  cx.on_render(|frame_ctx, content| {
    (content.selected_target.is_some()).then(|| {
      let masked_content = renderer.render_models(
        Box::new(IteratorAsHostRenderBatch(content.selected_target)),
        CameraRenderSource::Scene(content.main_camera),
        &HighLightMaskDispatcher,
        frame_ctx,
      );
      highlight.draw(frame_ctx, masked_content)
    })
  })
}
