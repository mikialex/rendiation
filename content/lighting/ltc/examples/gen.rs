use std::path::Path;

use rendiation_algebra::Vec4;
use rendiation_lighting_ltc::*;
use rendiation_texture_core::*;
use rendiation_texture_loader::*;

pub fn main() {
  let ltc_map = fit(GGX, &LtcFitConfig::default());

  write_image(&ltc_map.ltc_lut1, "ltc_1.png");
  write_image(&ltc_map.ltc_lut2, "ltc_2.png");
}

fn write_image(texture: &Texture2DBuffer<Vec4<f32>>, path: impl AsRef<Path>) {
  texture
    .map::<ImageLibContainerWrap<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>>(|pix| {
      image::Rgba([
        (pix.x.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.y.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.z.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.w.clamp(0.0, 1.0) * 255.0) as u8,
      ])
    })
    .0
    .save(path)
    .unwrap();
}
