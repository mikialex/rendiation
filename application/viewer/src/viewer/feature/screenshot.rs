use std::{io::Write, path::Path};

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
  let info = result.info();

  let mut png_encoder = png::Encoder::new(
    std::fs::File::create(png_output_path).unwrap(),
    info.width as u32,
    info.height as u32,
  );
  png_encoder.set_depth(png::BitDepth::Eight);
  png_encoder.set_color(png::ColorType::Rgba);

  let mut png_writer = png_encoder
    .write_header()
    .unwrap()
    .into_stream_writer_with_size(info.unpadded_bytes_per_row)
    .unwrap();

  match result.info().format {
    TextureFormat::Rgba8UnormSrgb => {
      let padded_buffer = result.read_raw();
      // from the padded_buffer we write just the unpadded bytes into the image
      for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
        png_writer
          .write_all(&chunk[..info.unpadded_bytes_per_row])
          .unwrap();
      }
      png_writer.finish().unwrap();
    }
    TextureFormat::Bgra8UnormSrgb => {
      let padded_buffer = result.read_raw();
      for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
        for [b, g, r, a] in chunk.array_chunks::<4>() {
          png_writer.write_all(&[*r, *g, *b, *a]).unwrap();
        }
      }
      png_writer.finish().unwrap();
    }
    _ => println!("unsupported format: {:?}", result.info().format),
  }
}
