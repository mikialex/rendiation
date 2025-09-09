use std::path::Path;

use crate::*;

pub fn use_enable_screenshot(cx: &mut ViewerCx) {
  cx.use_state_init(|cx| {
  cx.terminal.register_command("screenshot", |ctx, _parameters, _| {
    let result = ctx.renderer.read_next_render_result();

    async {
      match result.await {
          Ok(r) =>{
            if let Some(mut dir) = dirs::download_dir() {
              dir.push("screenshot.png"); // will override old but ok
              write_screenshot(&r, dir);
            }else {
              log::error!("failed to locate the system's default download directory to write viewer screenshot image")
            }
          },
          Err(e) => log::error!("{e:?}"),
      }
    }
  });

    ViewerScreenshot
  });
}

struct ViewerScreenshot;
impl CanCleanUpFrom<ViewerDropCx<'_>> for ViewerScreenshot {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    cx.terminal.unregister_command("screenshot");
  }
}

fn write_screenshot(result: &ReadableTextureBuffer, png_output_path: impl AsRef<Path>) {
  rendiation_texture_exporter::write_raw_gpu_buffer_image_as_png(
    &mut std::fs::File::create_buffered(png_output_path).unwrap(),
    Size::from_usize_pair_min_one((result.info().width, result.info().height)),
    &result.read_raw(),
    result.info().format,
    result.info().unpadded_bytes_per_row as u32,
    result.info().padded_bytes_per_row as u32,
  );
}
