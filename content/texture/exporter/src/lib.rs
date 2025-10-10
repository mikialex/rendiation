#![feature(iter_array_chunks)]

use std::io::Write;

use rendiation_texture_core::*;

pub fn write_gpu_buffer_image_as_png(target: impl Write, image: &GPUBufferImage) {
  write_raw_gpu_buffer_image_as_png(
    target,
    image.size,
    &image.data,
    image.format,
    image.bytes_per_row(),
    image.bytes_per_row(),
  )
}

/// the data may contains per row padding.
pub fn write_raw_gpu_buffer_image_as_png(
  target: impl Write,
  size: Size,
  data: &[u8],
  format: TextureFormat,
  unpadded_bytes_per_row: u32,
  padded_bytes_per_row: u32,
) {
  let (width, height) = size.into_u32();

  assert!(unpadded_bytes_per_row <= padded_bytes_per_row);
  assert!(padded_bytes_per_row * height == data.len() as u32);

  let mut png_encoder = png::Encoder::new(target, width, height);
  png_encoder.set_depth(png::BitDepth::Eight);
  png_encoder.set_color(png::ColorType::Rgba);

  let unpadded_bytes_per_row = unpadded_bytes_per_row as usize;
  let padded_bytes_per_row = padded_bytes_per_row as usize;

  let mut png_writer = png_encoder.write_header().unwrap();
  let mut png_writer = png_writer
    .stream_writer_with_size(unpadded_bytes_per_row)
    .unwrap();

  match format {
    TextureFormat::Rgba8UnormSrgb => {
      // from the padded_buffer we write just the unpadded bytes into the image
      for chunk in data.chunks(padded_bytes_per_row) {
        png_writer
          .write_all(&chunk[..unpadded_bytes_per_row])
          .unwrap();
      }
      png_writer.finish().unwrap();
    }
    TextureFormat::Bgra8UnormSrgb => {
      for chunk in data.chunks(padded_bytes_per_row) {
        for [b, g, r, a] in chunk[..unpadded_bytes_per_row].iter().array_chunks::<4>() {
          png_writer.write_all(&[*r, *g, *b, *a]).unwrap();
        }
      }
      png_writer.finish().unwrap();
    }
    _ => println!("unsupported format: {:?}", format),
  }
}
