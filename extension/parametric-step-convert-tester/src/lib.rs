mod gltf;
mod svg;

pub use gltf::*;
pub use svg::*;

use std::path::Path;

use rendiation_parametric_rendering::step::{read_parametric_rendering_data_from_step, StepReadConfig};

pub fn read_step(file: &Path) -> rendiation_parametric_rendering::step::StepConversionResult {
  let raw = std::fs::read_to_string(file).expect("failed to read STEP file");
  let config = StepReadConfig::default();
  read_parametric_rendering_data_from_step(&raw, config)
}
