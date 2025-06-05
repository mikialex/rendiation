use rendiation_texture_gpu_process::ToneMap;

use crate::*;

pub fn use_tonemap(cx: &mut Viewer3dRenderingCx) {
  let tonemap = cx.use_plain_state_init_by(|| ToneMap::new(cx.gpu));

  cx.on_render(|_| {
    // tonemap.update(frame_ctx.gpu);
  });
}
