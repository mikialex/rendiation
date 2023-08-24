#![feature(type_name_of_val)]

use rendiation_algebra::*;
use rendiation_texture::*;

mod brdf;
mod fit;

use std::path::Path;

use brdf::*;
use fit::*;

pub fn main() {
  // let ltc_map = fit::fit(GGX, &LtcFitConfig::default());
  // let ltc_map = fit::fit(
  //   GGX,
  //   &LtcFitConfig {
  //     lut_size: 32,
  //     sample_count: 32,
  //   },
  // );

  let ltc_map = fit::fit(
    GGX,
    &LtcFitConfig {
      lut_size: 8,
      sample_count: 32,
    },
  );

  write_image(&ltc_map.ltc_lut1, "ltc_1.png");
  write_image(&ltc_map.ltc_lut2, "ltc_2.png");
}

fn write_image(texture: &Texture2DBuffer<Vec4<f32>>, path: impl AsRef<Path>) {
  texture
    .map::<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>(|pix| {
      image::Rgba([
        (pix.x.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.y.clamp(0.0, 1.0) * 255.0) as u8,
        (pix.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
      ])
    })
    .save(path)
    .unwrap();
}
