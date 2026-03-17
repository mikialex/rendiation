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

pub fn create_gpu_texture_by_fn(
  size: Size,
  pixel: impl Fn(usize, usize) -> Vec4<f32>,
) -> GPUBufferImage {
  let mut data: Vec<u8> = vec![0; size.area() * 4];
  let s = size.into_usize();
  for y in 0..s.1 {
    for x in 0..s.0 {
      let pixel = pixel(x, y);
      data[(y * s.0 + x) * 4] = (255.).min(pixel.x * 255.) as u8;
      data[(y * s.0 + x) * 4 + 1] = (255.).min(pixel.y * 255.) as u8;
      data[(y * s.0 + x) * 4 + 2] = (255.).min(pixel.z * 255.) as u8;
      data[(y * s.0 + x) * 4 + 3] = (255.).min(pixel.w * 255.) as u8;
    }
  }

  GPUBufferImage {
    data,
    format: TextureFormat::Rgba8UnormSrgb,
    size,
  }
}
