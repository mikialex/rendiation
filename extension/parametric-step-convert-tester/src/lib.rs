mod gltf;
mod svg;

use std::path::Path;

pub use gltf::*;
use rendiation_parametric_rendering::step::{
  read_parametric_rendering_data_from_step, StepReadConfig,
};
pub use svg::*;

pub fn read_step(
  file: impl AsRef<Path>,
) -> rendiation_parametric_rendering::step::StepConversionResult {
  let raw = std::fs::read_to_string(file).expect("failed to read STEP file");
  let config = StepReadConfig::default();
  read_parametric_rendering_data_from_step(&raw, config)
}
