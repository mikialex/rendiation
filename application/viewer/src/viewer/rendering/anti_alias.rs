use rendiation_texture_gpu_process::{ReprojectInfo, TAAContent};

use crate::*;

pub fn use_viewer_taa(
  cx: &mut Viewer3dRenderingCx,
  renderer: Option<&dyn SceneRenderer<ContentKey = SceneContentKey>>,
  f: impl FnOnce(&mut FrameCtx, &FrameGeometryBuffer, &ReprojectInfo),
) -> Option<FrameGeometryBuffer> {
  let (cx, enable_taa) = cx.use_plain_state_init(&true);

  //   reproject: GPUReprojectInfo::new(gpu),

  let mut taa_content = SceneCameraTAAContent {
    queue: &cx.gpu.queue,
    camera: content.main_camera,
    renderer,
    f: |ctx: &mut FrameCtx| {
      let scene_result = attachment().use_hdr_if_enabled(hdr_enabled).request(ctx);
      let g_buffer = FrameGeometryBuffer::new(ctx);

      f(ctx);

      (
        TAAFrame {
          color: scene_result,
          depth: g_buffer.depth,
        },
        (g_buffer.entity_id, g_buffer.normal),
      )
    },
  };
}

struct SceneCameraTAAContent<'a, F> {
  renderer: &'a dyn SceneRenderer<ContentKey = SceneContentKey>,
  camera: EntityHandle<SceneCameraEntity>,
  queue: &'a GPUQueue,
  f: F,
}

impl<F, R> TAAContent<R> for SceneCameraTAAContent<'_, F>
where
  F: FnMut(&mut FrameCtx) -> (TAAFrame, R),
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    let cameras = self.renderer.get_camera_gpu();
    cameras.setup_camera_jitter(self.camera, next_jitter, self.queue);
  }

  fn render(&mut self, ctx: &mut FrameCtx) -> (TAAFrame, R) {
    (self.f)(ctx)
  }
}

pub fn use_fxaa(cx: &mut Viewer3dRenderingCx) {

  //   let maybe_aa_result = if self.enable_fxaa {
  //   let fxaa_target = maybe_aa_result.create_attachment_key().request(ctx);

  //   pass("fxaa")
  //     .with_color(&fxaa_target, store_full_frame())
  //     .render_ctx(ctx)
  //     .by(
  //       &mut FXAA {
  //         source: &maybe_aa_result,
  //       }
  //       .draw_quad(),
  //     );

  //   fxaa_target
  // } else {
  //   maybe_aa_result
  // };
}
