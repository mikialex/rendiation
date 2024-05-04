use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture_core::Texture2DBuffer;

mod brdf;
mod fit;
mod shader;

pub use brdf::*;
pub use fit::*;
pub use shader::*;

/// This const is a check flag to keep the shader impl sync with lut content generation.
/// User could use this version as their lut cache suffix to determine if the lut cache is invalid.
/// If any REAL lut generation logic changed, we will update this version.
///
/// 0: the original ltc impl
pub const LTC_LUT_VERSION: u64 = 0;
