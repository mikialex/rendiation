use rendiation_texture_gpu_process::SSAO;

use crate::*;

pub fn use_ssao<'a>(
  cx: &'a mut Viewer3dRenderingCx<'a>,
) -> (&'a mut Viewer3dRenderingCx<'a>, &'a mut SSAO) {
  let (cx, enable_ssao) = cx.use_plain_state_init(&true);
  let (cx, ssao) = cx.use_plain_state_init_by(|_| SSAO::new(todo!()));

  (cx, ssao)
}
