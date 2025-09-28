use rendiation_texture_gpu_base::create_gpu_texture2d;

use crate::*;

pub fn create_gpu_tex_from_png_buffer(
  cx: &GPU,
  buf: &[u8],
  format: TextureFormat,
) -> GPU2DTextureView {
  let buf = std::io::Cursor::new(buf);
  let png_decoder = png::Decoder::new(buf);
  let mut png_reader = png_decoder.read_info().unwrap();
  let mut buf = vec![0; png_reader.output_buffer_size().unwrap()];
  png_reader.next_frame(&mut buf).unwrap();

  let (width, height) = png_reader.info().size();
  create_gpu_texture2d(
    cx,
    &GPUBufferImage {
      data: buf,
      format,
      size: Size::from_u32_pair_min_one((width, height)),
    },
  )
}
