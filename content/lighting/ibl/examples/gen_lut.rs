use std::path::Path;

use rendiation_algebra::Vec4;
use rendiation_lighting_ibl::generate_brdf_lut;
use rendiation_texture_core::*;
use rendiation_texture_loader::*;
use rendiation_webgpu::*;

#[pollster::main]
pub async fn main() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let size = Size::from_u32_pair_min_one((64, 64));

  let pool = Default::default();
  let mut frame_ctx = FrameCtx::new(&gpu, size, &pool);

  let target =
    create_empty_2d_texture_view(&gpu, size, TextureUsages::all(), TextureFormat::Rgba8Unorm);
  let target_res = target.resource.clone();

  generate_brdf_lut(&mut frame_ctx, target);

  frame_ctx.final_submit();

  let mut encoder = gpu.device.create_encoder();
  let reader = encoder.read_texture_2d(
    &gpu.device,
    &GPU2DTexture::try_from(target_res).unwrap(),
    ReadRange {
      size,
      offset_x: 0,
      offset_y: 0,
    },
  );
  gpu.submit_encoder(encoder);
  let result = reader.await.unwrap();

  let padded_buffer = result.read_raw();
  let info = result.info();
  let mut buffer = Vec::with_capacity(info.unpadded_bytes_per_row * info.height);
  for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
    buffer.extend_from_slice(&chunk[..info.unpadded_bytes_per_row]);
  }

  let image: &[Vec4<u8>] = bytemuck::cast_slice(&buffer);
  let image = Texture2DBuffer {
    data: image.to_vec(),
    size,
  };

  write_image(&image, "brdf_lut.png");
}

fn write_image(texture: &Texture2DBuffer<Vec4<u8>>, path: impl AsRef<Path>) {
  texture
    .map::<ImageLibContainerWrap<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>(|pix| {
      image::Rgba([pix.x, pix.y, pix.z, pix.w])
    })
    .0
    .save(path)
    .unwrap();
}
