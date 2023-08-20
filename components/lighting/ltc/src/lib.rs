use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture::Texture2DBuffer;

mod brdf;
mod fit;
mod shader;

pub use fit::*;
pub use shader::*;

/// This const is a check flag to keep the shader impl sync with lut content generation.
/// User could use this version as their lut cache suffix to determine if the cache is slate.
/// If any REAL lut generation logic changed, we will update this version.
pub const LTC_LUT_VERSION: u64 = 0;
