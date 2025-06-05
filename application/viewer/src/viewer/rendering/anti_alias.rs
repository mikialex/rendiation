use rendiation_texture_gpu_process::*;

use crate::*;

pub fn use_viewer_taa(
  cx: &mut Viewer3dRenderingCx,
  f: impl FnOnce(&mut FrameCtx, &FrameGeometryBuffer, &ReprojectInfo),
) -> Option<FrameGeometryBuffer> {
  let (cx, enable_taa) = cx.use_plain_state_init(&true);
  let (cx, reproject) = cx.use_gpu_state(GPUReprojectInfo::new);

  //  let (
  //   TAAFrame {
  //     color: maybe_aa_result,
  //     depth: scene_depth,
  //   },
  //   (id_buffer, normal_buffer),
  // ) = if self.enable_taa {
  //   self
  //     .taa
  //     .render_aa_content(taa_content, ctx, &self.reproject)
  // } else {
  //   taa_content.render(ctx)
  // };

  cx.on_render(|frame, content| {
    let mut taa_content = SceneCameraTAAContent {
      queue: &frame.gpu.queue,
      camera: todo!(),
      f: |ctx: &mut FrameCtx| {
        let scene_result = attachment().use_hdr_if_enabled(hdr_enabled).request(ctx);
        let g_buffer = FrameGeometryBuffer::new(ctx);

        f(frame, &g_buffer, &reproject.reproject.get());

        (
          TAAFrame {
            color: scene_result,
            depth: g_buffer.depth,
          },
          (g_buffer.entity_id, g_buffer.normal),
        )
      },
    };

    let taa_results = taa_content.render(frame);

    FrameGeometryBuffer {
      depth: taa_results.0.depth,
      normal: taa_results.1 .1,
      entity_id: taa_results.1 .0,
    }
  })
}

struct SceneCameraTAAContent<'a, F> {
  camera: &'a CameraGPU,
  queue: &'a GPUQueue,
  f: F,
}

impl<F, R> TAAContent<R> for SceneCameraTAAContent<'_, F>
where
  F: FnMut(&mut FrameCtx) -> (TAAFrame, R),
{
  fn set_jitter(&mut self, next_jitter: Vec2<f32>) {
    // let cameras = self.renderer.get_camera_gpu();
    // self.camera.setup_camera_jitter(self.camera, next_jitter, self.queue);
  }

  fn render(&mut self, ctx: &mut FrameCtx) -> (TAAFrame, R) {
    (self.f)(ctx)
  }
}

pub fn use_fxaa(cx: &mut Viewer3dRenderingCx) {
  let (cx, enable_fxaa) = cx.use_plain_state_init(&false);

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
