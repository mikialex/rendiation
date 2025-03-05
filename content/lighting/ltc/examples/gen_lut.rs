use std::{io::Write, path::Path};

use half::f16;
use rendiation_algebra::Vec4;
use rendiation_lighting_ltc::*;
use rendiation_texture_core::*;

pub fn main() {
  let ltc_map = fit(GGXxLTCxFit, &LtcFitConfig::default());

  write_image(&ltc_map.ltc_lut1, "ltc_1.bin");
  write_image(&ltc_map.ltc_lut2, "ltc_2.bin");
}

fn write_image(texture: &Texture2DBuffer<Vec4<f32>>, path: impl AsRef<Path>) {
  let mut file = std::fs::File::create(path).unwrap();
  let buffer = texture
    .as_buffer()
    .iter()
    .map(|v| v.map(f16::from_f32))
    .collect::<Vec<_>>();
  file
    .write_all(bytemuck::cast_slice(buffer.as_slice()))
    .unwrap();
}
