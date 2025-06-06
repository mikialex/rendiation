use rendiation_texture_gpu_process::SSAO;

use crate::*;

pub fn use_ssao<'a>(
  cx: &'a mut Viewer3dRenderingCx<'a>,
) -> (&'a mut Viewer3dRenderingCx<'a>, &'a mut SSAO) {
  let (cx, enable_ssao) = cx.use_plain_state_init(&true);
  let (cx, ssao) = cx.use_gpu_state(SSAO::new);

  //   if self.enable_ssao {
  //   let ao = self.ssao.draw(
  //     ctx,
  //     &g_buffer.depth,
  //     &self.reproject.reproject,
  //     reversed_depth,
  //   );

  //   pass("ao blend to scene")
  //     .with_color(&scene_result, load_and_store())
  //     .render_ctx(ctx)
  //     .by(&mut copy_frame(
  //       ao,
  //       BlendState {
  //         color: BlendComponent {
  //           src_factor: BlendFactor::Dst,
  //           dst_factor: BlendFactor::Zero,
  //           operation: BlendOperation::Add,
  //         },
  //         alpha: BlendComponent::REPLACE,
  //       }
  //       .into(),
  //     ));
  // }

  (cx, ssao)
}
