use rendiation_texture_gpu_process::*;

use crate::*;

pub fn use_high_lighter(
  cx: &mut Viewer3dRenderingCx,
  renderer: Option<&dyn SceneRenderer>,
) -> Option<impl PassContent> {
  let (cx, highlight) = cx.use_gpu_state(HighLighter::new);

  cx.on_render(|cx| {
    (cx.content.selected_target.is_some()).then(|| {
      let masked_content = renderer.render_models(
        Box::new(IteratorAsHostRenderBatch(cx.content.selected_target)),
        CameraRenderSource::Scene(cx.content.main_camera),
        &HighLightMaskDispatcher,
        cx.frame,
      );
      highlight.draw(frame_ctx, masked_content)
    })
  })
}
