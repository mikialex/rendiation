use std::path::Path;

use crate::*;

pub const CMD_SCREENSHOT: &str = "screenshot";

pub fn use_enable_screenshot(cx: &mut ViewerCx) {
  cx.use_state_init(|cx| {
    cx.terminal.register_command(CMD_SCREENSHOT, |ctx, parameters, _| {
      let viewport_id = parameters.iter().nth(1).and_then(|v|v.parse::<u64>().ok()).unwrap_or_else(||{
        log::warn!("viewport not specified, using the any viewport in current viewer");
        *ctx.renderer.views.iter().next().unwrap().0
      });
      let result = ctx.renderer.views.get_mut(&viewport_id).unwrap().read_next_render_result();

      async {
        match result.await {
            Ok(r) =>{
              // todo, support download in web
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
    cx.terminal.unregister_command(CMD_SCREENSHOT);
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
